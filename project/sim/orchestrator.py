import argparse
import json
import os
import random
import sys
import threading
import time
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Dict, List, Optional, Tuple

# Nos: nome logico -> URL HTTP/RPC publicada no host pelo compose.
NODES = {
    "node1": "http://127.0.0.1:7101/rpc",
    "node2": "http://127.0.0.1:7102/rpc",
    "node3": "http://127.0.0.1:7103/rpc",
    "node4": "http://127.0.0.1:7104/rpc",
}

# Caminho da carteira de minerador.
# IMPORTANTE: as chamadas RPC de wallet passam por um sandbox que enraiza o
# path em WALLET_KEYS_DIR (/data/keys no compose) e REJEITA paths absolutos.
# Por isso enviamos o nome relativo "miner_wallet.json"; o sandbox o resolve
# para /data/keys/miner_wallet.json, que e exatamente onde o minerador
# (MINER_WALLET_SEED_PATH no compose) procura a carteira.
# A senha deve bater com MINER_WALLET_PASSWORD no docker-compose.sim.yml.
MINER_KEY_PATH = "miner_wallet.json"
MINER_PASSWORD = "miner123"

DEFAULT_FEE = 10000
SEND_UNIT = 100000

ACTIONS = [
    ("mine", 0.55),
    ("send", 0.30),
    ("query", 0.15),
]

_rpc_id = 0
_rpc_id_lock = threading.Lock()


def next_rpc_id():
    """Gera IDs JSON-RPC unicos mesmo com chamadas paralelas."""
    global _rpc_id
    with _rpc_id_lock:
        _rpc_id += 1
        return _rpc_id


def ts():
    return datetime.now(timezone.utc).strftime("%H:%M:%S")


def log(msg):
    print(f"[{ts()}] {msg}", flush=True)


def iso_now():
    return datetime.now(timezone.utc).isoformat()


def categorize_error(error):
    if not error:
        return None
    msg = str(error).lower()
    if "previous block hash does not match" in msg:
        return "previous_block_hash_mismatch"
    if "not valid utxos" in msg or "invalid utxo" in msg:
        return "invalid_utxo"
    if "insufficient funds" in msg or "sem saldo" in msg:
        return "insufficient_funds"
    if "wallet" in msg or "senha" in msg or "keystore" in msg:
        return "wallet_error"
    if "transport:" in msg:
        return "transport_error"
    if "decode:" in msg:
        return "decode_error"
    if "sem endereco destino" in msg:
        return "missing_destination"
    return "other"


def directory_size_bytes(path):
    total = 0
    if not os.path.exists(path):
        return 0
    for root, _, files in os.walk(path):
        for filename in files:
            file_path = os.path.join(root, filename)
            try:
                total += os.path.getsize(file_path)
            except OSError:
                pass
    return total


@dataclass
class ActionPlan:
    action: str
    node: str
    dst: Optional[str] = None
    seed: int = 0


class RpcClient:
    def __init__(self, url: str):
        self.url = url

    def call(
        self, method: str, params=None, timeout: int = 15
    ) -> Tuple[object, Optional[str]]:
        """Faz uma chamada JSON-RPC 2.0 e devolve (result, error)."""
        payload = {"jsonrpc": "2.0", "id": next_rpc_id(), "method": method}
        if params is not None:
            payload["params"] = params
        data = json.dumps(payload).encode("utf-8")
        req = urllib.request.Request(
            self.url, data=data, headers={"Content-Type": "application/json"}
        )
        try:
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                body = json.loads(resp.read().decode("utf-8"))
        except urllib.error.URLError as e:
            return None, f"transport: {e}"
        except json.JSONDecodeError as e:
            return None, f"decode: {e}"
        if isinstance(body, dict) and body.get("error"):
            return None, body["error"]
        return (body.get("result") if isinstance(body, dict) else body), None


class NodeAgent:
    def __init__(self, name: str, url: str, wallet_path: str, wallet_password: str):
        self.name = name
        self.rpc = RpcClient(url)
        self.wallet_path = wallet_path
        self.wallet_password = wallet_password
        self.address = None

    def status(self):
        return self.rpc.call("node_status")

    def ensure_wallet(self) -> bool:
        """Cria a carteira do minerador ou importa uma ja existente."""
        res, err = self.rpc.call(
            "wallet_new",
            {"password": self.wallet_password, "path": self.wallet_path},
        )
        if res and res.get("address"):
            self.address = res["address"]
            log(f"  {self.name}: carteira criada {self.address[:16]}...")
            return True

        # Ja existe -> importa para recuperar o endereco.
        res2, err2 = self.rpc.call(
            "wallet_import",
            {"password": self.wallet_password, "path": self.wallet_path},
        )
        if res2 and res2.get("address"):
            self.address = res2["address"]
            log(f"  {self.name}: carteira ja existia {self.address[:16]}...")
            return True

        log(f"  {self.name}: FALHA ao criar/importar carteira: {err or err2}")
        return False

    def balance(self) -> Tuple[Optional[int], Optional[str]]:
        res, err = self.rpc.call(
            "wallet_balance",
            {"key_path": self.wallet_path, "password": self.wallet_password},
        )
        if err:
            return None, str(err)
        if not isinstance(res, dict):
            return None, "wallet_balance retornou resposta invalida"
        return int(res.get("balance") or 0), None

    def mine(self, fallback_from=None, fallback_reason=None, balance_before=None):
        started = time.time()
        res, err = self.rpc.call("mine_block", timeout=120)
        end = time.time()
        result = {
            "action": "mine",
            "node": self.name,
            "ok": err is None,
            "duration_seconds": round(end - started, 3),
        }
        if fallback_from:
            result["fallback_from"] = fallback_from
        if fallback_reason:
            result["fallback_reason"] = fallback_reason
        if balance_before is not None:
            result["balance_before"] = balance_before

        if err:
            result["error"] = str(err)
            result["error_category"] = categorize_error(err)
            log(f"  acao=mine no={self.name} -> ERRO: {err}")
        else:
            log(f"  acao=mine no={self.name} -> ok (resp recebida)")
        return result

    def query_mempool(self):
        started = time.time()
        res, err = self.rpc.call("node_mempool")
        count = (res or {}).get("count") if res else None
        log(f"  acao=query no={self.name} -> mempool count={count}")
        result = {
            "action": "query",
            "node": self.name,
            "mempool": count,
            "ok": err is None,
            "duration_seconds": round(time.time() - started, 3),
        }
        if err:
            result["error"] = str(err)
            result["error_category"] = categorize_error(err)
        return result

    def send_to(self, dst_node, rng: random.Random):
        started = time.time()
        balance_before, balance_err = self.balance()
        if balance_err:
            log(
                f"  acao=send {self.name}->{dst_node.name} -> falha ao consultar saldo: {balance_err}"
            )
            return {
                "action": "send",
                "src": self.name,
                "dst": dst_node.name,
                "ok": False,
                "error": balance_err,
                "error_category": categorize_error(balance_err),
                "duration_seconds": round(time.time() - started, 3),
            }

        max_amount = balance_before - DEFAULT_FEE
        if max_amount <= 0:
            log(
                f"  acao=send {self.name}->{dst_node.name} -> sem saldo "
                f"(balance={balance_before}); fallback=mine"
            )
            return self.mine(
                fallback_from="send",
                fallback_reason="insufficient_balance",
                balance_before=balance_before,
            )

        max_units = max_amount // SEND_UNIT
        if max_units > 0:
            amount = rng.randint(1, max_units) * SEND_UNIT
        else:
            amount = max_amount

        params = {
            "from": {"key_path": self.wallet_path, "password": self.wallet_password},
            "to": dst_node.address,
            "amount": amount,
            "fee": DEFAULT_FEE,
            "message": "sim-tx",
        }
        res, err = self.rpc.call("wallet_send", params)
        ok = bool(res and res.get("success"))
        detail = (res or {}).get("error") if not ok else (res or {}).get("tx_id", "")
        if err:
            ok = False
            detail = err

        log(
            f"  acao=send {self.name}->{dst_node.name} amt={amount} "
            f"balance={balance_before} -> {'ok' if ok else 'falha'}: {str(detail)[:40]}"
        )
        result = {
            "action": "send",
            "src": self.name,
            "dst": dst_node.name,
            "amount": amount,
            "fee": DEFAULT_FEE,
            "balance_before": balance_before,
            "ok": ok,
            "duration_seconds": round(time.time() - started, 3),
        }
        if ok:
            result["tx_id"] = detail
        else:
            result["error"] = str(detail)
            result["error_category"] = categorize_error(detail)
        return result

    def run_action(self, plan: ActionPlan, nodes_by_name: Dict[str, "NodeAgent"]):
        rng = random.Random(plan.seed)
        try:
            if plan.action == "mine":
                return self.mine()
            if plan.action == "send":
                dst_node = nodes_by_name.get(plan.dst)
                if not dst_node or not dst_node.address:
                    error = "sem endereco destino"
                    return {
                        "action": "send",
                        "src": self.name,
                        "dst": plan.dst,
                        "ok": False,
                        "error": error,
                        "error_category": categorize_error(error),
                    }
                return self.send_to(dst_node, rng)
            if plan.action == "query":
                return self.query_mempool()
            error = "acao desconhecida"
            return {
                "action": plan.action,
                "node": self.name,
                "ok": False,
                "error": error,
                "error_category": categorize_error(error),
            }
        except Exception as e:
            error = repr(e)
            return {
                "action": plan.action,
                "node": self.name,
                "ok": False,
                "error": error,
                "error_category": categorize_error(error),
            }


class SimulationOrchestrator:
    def __init__(self, nodes: List[NodeAgent], args):
        self.nodes = nodes
        self.nodes_by_name = {node.name: node for node in nodes}
        self.rng = random.Random(args.seed)
        self.rounds = args.rounds
        self.seed = args.seed
        self.delay = args.delay
        self.settle = args.settle
        self.out = args.out
        self.history = []

    def wait_until_up(self, retries=30, delay=2):
        """Espera todos os nos responderem node_status."""
        log("Aguardando os 4 nos ficarem prontos...")
        for attempt in range(retries):
            ready = []
            for node in self.nodes:
                res, err = node.status()
                if res is not None:
                    ready.append(node.name)
            if len(ready) == len(self.nodes):
                log(f"Todos prontos: {', '.join(ready)}")
                return True
            log(f"  tentativa {attempt + 1}/{retries}: prontos={ready}")
            time.sleep(delay)
        return False

    def ensure_wallets(self):
        """Cria a carteira de minerador em cada no (idempotente)."""
        log("Garantindo carteira de minerador em cada no...")
        addresses = {}
        for node in self.nodes:
            if node.ensure_wallet():
                addresses[node.name] = node.address
        return addresses

    def snapshot(self):
        """Coleta altura, top-hash, peers e saldo de cada no."""
        snap = {"timestamp": iso_now(), "nodes": {}}
        for node in self.nodes:
            res, err = node.status()
            balance, balance_err = node.balance()
            if res is None:
                node_snap = {"error": str(err)}
            else:
                node_snap = {
                    "height": res.get("block_height"),
                    "top": (res.get("top_block_hash") or "")[:16],
                    "peers": res.get("peers_connected"),
                }
            if balance_err:
                node_snap["balance_error"] = balance_err
            else:
                node_snap["balance"] = balance
            snap["nodes"][node.name] = node_snap
        snap["converged"] = self.is_converged(snap)
        return snap

    def is_converged(self, snap):
        """True se todos os nos tem a mesma altura e mesmo top-hash."""
        tops = set()
        heights = set()
        for v in snap.get("nodes", snap).values():
            if "error" in v:
                return False
            tops.add(v["top"])
            heights.add(v["height"])
        return len(tops) == 1 and len(heights) == 1

    def print_snapshot(self, snap):
        parts = []
        for node in self.nodes:
            v = snap.get("nodes", snap)[node.name]
            if "error" in v:
                parts.append(f"{node.name}=ERR")
            else:
                balance = v.get("balance", "?")
                parts.append(
                    f"{node.name}=h{v['height']}/{v['top']}/p{v['peers']}/bal{balance}"
                )
        flag = "CONVERGIDO" if self.is_converged(snap) else "divergente"
        log(f"  estado: {' | '.join(parts)}  [{flag}]")

    def pick_action(self):
        r = self.rng.random()
        acc = 0.0
        for action, weight in ACTIONS:
            acc += weight
            if r <= acc:
                return action
        return ACTIONS[-1][0]

    def plan_round(self) -> List[ActionPlan]:
        plans = []
        node_names = [node.name for node in self.nodes]
        for node in self.nodes:
            action = self.pick_action()
            dst = None
            if action == "send":
                dst = self.rng.choice(
                    [name for name in node_names if name != node.name]
                )
            plans.append(
                ActionPlan(
                    action=action,
                    node=node.name,
                    dst=dst,
                    seed=self.rng.randint(0, 2**31 - 1),
                )
            )
        return plans

    def run_round(self, round_number: int):
        round_started = time.time()
        round_started_at = iso_now()
        plans = self.plan_round()
        plan_desc = ", ".join(
            f"{p.node}:{p.action}->{p.dst}" if p.dst else f"{p.node}:{p.action}"
            for p in plans
        )
        log(f"Rodada {round_number}/{self.rounds}: {plan_desc}")

        events_by_node = {}
        with ThreadPoolExecutor(max_workers=len(self.nodes)) as executor:
            future_to_plan = {
                executor.submit(
                    self.nodes_by_name[plan.node].run_action,
                    plan,
                    self.nodes_by_name,
                ): plan
                for plan in plans
            }
            for future in as_completed(future_to_plan):
                plan = future_to_plan[future]
                try:
                    events_by_node[plan.node] = future.result()
                except Exception as e:
                    error = repr(e)
                    events_by_node[plan.node] = {
                        "action": plan.action,
                        "node": plan.node,
                        "ok": False,
                        "error": error,
                        "error_category": categorize_error(error),
                    }

        events = [events_by_node[node.name] for node in self.nodes]
        time.sleep(self.delay)
        snap = self.snapshot()
        self.print_snapshot(snap)
        self.history.append(
            {
                "round": round_number,
                "started_at": round_started_at,
                "ended_at": iso_now(),
                "duration_seconds": round(time.time() - round_started, 3),
                "events": events,
                "snapshot": snap,
            }
        )

    def event_node_name(self, event):
        return event.get("node") or event.get("src")

    def summarize_events(self):
        stats = {
            "event_counts": {
                "mine": 0,
                "mine_ok": 0,
                "mine_failed": 0,
                "send": 0,
                "send_ok": 0,
                "send_failed": 0,
                "send_fallback_to_mine": 0,
                "query": 0,
                "query_failed": 0,
                "failed": 0,
            },
            "blocks_mined_by_node": {node.name: 0 for node in self.nodes},
            "mine_failures_by_node": {node.name: 0 for node in self.nodes},
            "tx_by_node": {
                node.name: {"submitted": 0, "accepted": 0, "failed": 0}
                for node in self.nodes
            },
            "tx_failures_by_category": {},
            "failures_by_category": {},
            "actions_by_node": {
                node.name: {"mine": 0, "send": 0, "query": 0, "failed": 0}
                for node in self.nodes
            },
        }

        for item in self.history:
            for event in item.get("events", []):
                action = event.get("action")
                node_name = self.event_node_name(event)
                if (
                    node_name in stats["actions_by_node"]
                    and action in stats["actions_by_node"][node_name]
                ):
                    stats["actions_by_node"][node_name][action] += 1

                if action == "mine":
                    stats["event_counts"]["mine"] += 1
                    if event.get("fallback_from") == "send":
                        stats["event_counts"]["send_fallback_to_mine"] += 1
                    if event.get("ok"):
                        stats["event_counts"]["mine_ok"] += 1
                        if node_name in stats["blocks_mined_by_node"]:
                            stats["blocks_mined_by_node"][node_name] += 1
                    else:
                        stats["event_counts"]["mine_failed"] += 1
                        if node_name in stats["mine_failures_by_node"]:
                            stats["mine_failures_by_node"][node_name] += 1
                elif action == "send":
                    stats["event_counts"]["send"] += 1
                    if node_name in stats["tx_by_node"]:
                        stats["tx_by_node"][node_name]["submitted"] += 1
                    if event.get("ok"):
                        stats["event_counts"]["send_ok"] += 1
                        if node_name in stats["tx_by_node"]:
                            stats["tx_by_node"][node_name]["accepted"] += 1
                    else:
                        stats["event_counts"]["send_failed"] += 1
                        if node_name in stats["tx_by_node"]:
                            stats["tx_by_node"][node_name]["failed"] += 1
                        category = event.get("error_category") or categorize_error(
                            event.get("error")
                        )
                        stats["tx_failures_by_category"][category] = (
                            stats["tx_failures_by_category"].get(category, 0) + 1
                        )
                elif action == "query":
                    stats["event_counts"]["query"] += 1
                    if not event.get("ok"):
                        stats["event_counts"]["query_failed"] += 1

                if event.get("ok") is False:
                    stats["event_counts"]["failed"] += 1
                    if node_name in stats["actions_by_node"]:
                        stats["actions_by_node"][node_name]["failed"] += 1
                    category = event.get("error_category") or categorize_error(
                        event.get("error")
                    )
                    stats["failures_by_category"][category] = (
                        stats["failures_by_category"].get(category, 0) + 1
                    )

        stats["transactions"] = {
            "total_submitted": stats["event_counts"]["send"],
            "accepted": stats["event_counts"]["send_ok"],
            "failed": stats["event_counts"]["send_failed"],
            "failures_by_category": stats["tx_failures_by_category"],
        }
        return stats

    def disk_usage(self):
        project_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        sim_data_dir = os.path.join(project_dir, ".sim-data")
        by_node = {}
        for node in self.nodes:
            by_node[node.name] = directory_size_bytes(
                os.path.join(sim_data_dir, node.name)
            )
        return {
            "path": sim_data_dir,
            "total_bytes": directory_size_bytes(sim_data_dir),
            "by_node_bytes": by_node,
        }

    def snapshot_nodes(self, item):
        snap = item.get("snapshot", {})
        return snap.get("nodes", snap)

    def height_for_item(self, item):
        nodes = self.snapshot_nodes(item)
        heights = [
            v.get("height")
            for v in nodes.values()
            if isinstance(v, dict) and "height" in v
        ]
        if not heights:
            return None
        return min(heights)

    def block_time_stats(self):
        samples = []
        previous_height = None
        previous_time = None
        for item in self.history:
            if "snapshot" not in item:
                continue
            height = self.height_for_item(item)
            timestamp = item.get("snapshot", {}).get("timestamp")
            if height is None or not timestamp:
                continue
            current_time = datetime.fromisoformat(timestamp).timestamp()
            if previous_height is not None and height > previous_height:
                delta_height = height - previous_height
                delta_time = current_time - previous_time
                samples.append(
                    {
                        "from_height": previous_height,
                        "to_height": height,
                        "blocks": delta_height,
                        "seconds": round(delta_time, 3),
                        "seconds_per_block": round(delta_time / delta_height, 3),
                    }
                )
            previous_height = height
            previous_time = current_time

        total_blocks = sum(sample["blocks"] for sample in samples)
        total_seconds = sum(sample["seconds"] for sample in samples)
        avg = round(total_seconds / total_blocks, 3) if total_blocks else None
        return {
            "average_seconds_between_blocks": avg,
            "observed_blocks": total_blocks,
            "samples": samples,
        }

    def convergence_stats(self):
        divergent_rounds = []
        for item in self.history:
            snap = item.get("snapshot")
            if snap and not snap.get("converged", self.is_converged(snap)):
                divergent_rounds.append(item.get("round"))
        return {
            "divergent_rounds": divergent_rounds,
            "divergent_round_count": len(divergent_rounds),
        }

    def final_balances(self, snap):
        return {
            name: node_snap.get("balance")
            for name, node_snap in snap.get("nodes", snap).items()
        }

    def final_peers(self, snap):
        return {
            name: node_snap.get("peers")
            for name, node_snap in snap.get("nodes", snap).items()
        }

    def run(self):
        log(
            f"=== Simulacao distribuida Caramuru | seed={self.seed} rounds={self.rounds} ==="
        )

        if not self.wait_until_up():
            log("ERRO: nem todos os nos ficaram prontos. Abortando.")
            sys.exit(1)

        self.ensure_wallets()
        time.sleep(2)

        simulation_started = time.time()
        simulation_started_at = iso_now()

        log("Estado inicial:")
        snap0 = self.snapshot()
        self.print_snapshot(snap0)
        self.history.append(
            {"round": 0, "event": "initial", "timestamp": iso_now(), "snapshot": snap0}
        )

        for i in range(1, self.rounds + 1):
            self.run_round(i)

        log(f"Aguardando {self.settle}s para convergencia final...")
        time.sleep(self.settle)
        snap_final = self.snapshot()
        log("Estado final:")
        self.print_snapshot(snap_final)
        converged = self.is_converged(snap_final)
        simulation_elapsed = round(time.time() - simulation_started, 3)
        self.history.append(
            {
                "round": self.rounds + 1,
                "event": "final",
                "timestamp": iso_now(),
                "snapshot": snap_final,
            }
        )

        event_stats = self.summarize_events()
        summary = {
            "seed": self.seed,
            "rounds": self.rounds,
            "started_at": simulation_started_at,
            "elapsed_seconds": simulation_elapsed,
            "converged_final": converged,
            "final_snapshot": snap_final,
            "event_counts": event_stats["event_counts"],
            "events": event_stats,
            "transactions": event_stats["transactions"],
            "blocks_mined_by_node": event_stats["blocks_mined_by_node"],
            "balances_final": self.final_balances(snap_final),
            "peers_final": self.final_peers(snap_final),
            "convergence": self.convergence_stats(),
            "block_time": self.block_time_stats(),
            "disk_usage": self.disk_usage(),
            "generated_at": iso_now(),
        }
        report = {"summary": summary, "history": self.history}
        with open(self.out, "w", encoding="utf-8") as f:
            json.dump(report, f, indent=2, ensure_ascii=False)

        log(f"Relatorio salvo em {self.out}")
        log(f"Convergencia final: {'SIM' if converged else 'NAO'}")
        sys.exit(0 if converged else 2)


def parse_args():
    ap = argparse.ArgumentParser(
        description="Orquestrador da simulacao distribuida do Caramuru"
    )
    ap.add_argument("--rounds", type=int, default=30, help="numero de rodadas de acao")
    ap.add_argument(
        "--seed", type=int, default=42, help="seed fixa para reprodutibilidade"
    )
    ap.add_argument("--delay", type=float, default=1.5, help="pausa (s) entre rodadas")
    ap.add_argument(
        "--settle", type=float, default=8.0, help="espera final (s) para convergencia"
    )
    ap.add_argument(
        "--out", default="sim/results.json", help="arquivo de saida das metricas"
    )
    return ap.parse_args()


def main():
    args = parse_args()
    nodes = [
        NodeAgent(name, url, MINER_KEY_PATH, MINER_PASSWORD)
        for name, url in NODES.items()
    ]
    SimulationOrchestrator(nodes, args).run()


if __name__ == "__main__":
    main()
