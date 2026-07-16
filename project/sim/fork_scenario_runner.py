import argparse
import json
import time
from typing import Dict, List, Optional, Tuple

from sim_common import (
    MINER_KEY_PATH,
    MINER_PASSWORD,
    NODES,
    NodeEndpoint,
    RpcClient,
    RpcError,
    ensure_miner_wallet,
    log,
)


class ForkScenarioRunner:
    def __init__(
        self,
        nodes: Dict[str, NodeEndpoint] = NODES,
        pause: bool = False,
        step_delay: float = 0.0,
        rpc_timeout: int = 20,
        mine_timeout: int = 180,
        wallet_path: str = MINER_KEY_PATH,
        wallet_password: str = MINER_PASSWORD,
    ):
        self.nodes = nodes
        self.pause_enabled = pause
        self.step_delay = step_delay
        self.mine_timeout = mine_timeout
        self.wallet_path = wallet_path
        self.wallet_password = wallet_password
        self.wallet_addresses: Dict[str, str] = {}
        self.clients = {
            name: RpcClient(endpoint.rpc_url, timeout=rpc_timeout)
            for name, endpoint in nodes.items()
        }

    def title(self, message: str):
        print("\n" + "=" * 80)
        print(message)
        print("=" * 80)

    def step(self, message: str):
        log(f"PASSO: {message}")
        if self.pause_enabled:
            input("Pressione Enter para continuar...")
        elif self.step_delay > 0:
            time.sleep(self.step_delay)

    def wait(self, seconds: float, reason: str = "aguardando propagação"):
        log(f"Aguardando {seconds:.1f}s ({reason})")
        time.sleep(seconds)

    def wait_until_up(self, timeout_seconds: float = 30.0):
        deadline = time.time() + timeout_seconds
        pending = set(self.nodes.keys())
        while pending and time.time() < deadline:
            for name in list(pending):
                try:
                    self.status(name)
                    pending.remove(name)
                    log(f"{name} online")
                except RpcError:
                    pass
            if pending:
                time.sleep(0.5)

        if pending:
            raise RuntimeError(
                f"Nós não responderam a tempo: {', '.join(sorted(pending))}"
            )

    def ensure_wallet(self, node: str) -> str:
        self._check_node(node)
        address, error = ensure_miner_wallet(
            self.clients[node],
            node,
            wallet_path=self.wallet_path,
            wallet_password=self.wallet_password,
        )
        if error or not address:
            raise RuntimeError(f"Falha ao configurar carteira de {node}: {error}")
        self.wallet_addresses[node] = address
        return address

    def ensure_wallets(self) -> Dict[str, str]:
        log("Garantindo carteira de minerador em cada nó...")
        for node in sorted(self.nodes):
            self.ensure_wallet(node)
        return self.wallet_addresses

    def status(self, node: str) -> dict:
        self._check_node(node)
        return self.clients[node].call_or_raise("node_status")

    def genesis_hash(self, node: str) -> str:
        return str(self.status(node).get("genesis_hash", ""))

    def bootstrap_common_root(
        self,
        bootstrap_node: str = "node1",
        timeout_seconds: float = 40.0,
    ) -> str:
        """Garante que todos os nós compartilhem o mesmo bloco raiz (genesis).

        Antes de qualquer partição é preciso que todos os nós tenham pelo menos
        um bloco e que esse bloco seja idêntico. Caso contrário, cada grupo
        minera seu próprio genesis durante a partição, os hashes divergem e o
        handshake P2P rejeita a reconexão com "Network mismatch".
        """
        self._check_node(bootstrap_node)
        empty = "0" * 64

        genesis = {name: self.genesis_hash(name) for name in sorted(self.nodes)}
        non_empty = {g for g in genesis.values() if g and g != empty}

        if len(non_empty) > 1:
            details = ", ".join(f"{n}={g[:12]}" for n, g in sorted(genesis.items()))
            raise RuntimeError(
                "Os nós já possuem blocos raiz divergentes: "
                f"{details}. Resete os volumes antes de rodar o cenário "
                "(docker compose -f docker-compose.sim.yml down -v)."
            )

        if len(non_empty) == 1 and all(
            g == next(iter(non_empty)) for g in genesis.values()
        ):
            root = next(iter(non_empty))
            log(f"OK: todos os nós já compartilham o bloco raiz {root[:16]}")
            return root

        log(
            "Nós sem raiz comum; minerando bloco inicial em "
            f"{bootstrap_node} para fixar um genesis compartilhado..."
        )
        self.connect_default_topology()
        self.mine(bootstrap_node)
        self.wait_until_converged(timeout_seconds=timeout_seconds)

        genesis = {name: self.genesis_hash(name) for name in sorted(self.nodes)}
        roots = {g for g in genesis.values()}
        if len(roots) != 1 or empty in roots:
            details = ", ".join(f"{n}={g[:12]}" for n, g in sorted(genesis.items()))
            raise RuntimeError(
                f"Falha ao fixar bloco raiz comum após o bootstrap: {details}"
            )

        root = next(iter(roots))
        log(f"OK: bloco raiz comum fixado em {root[:16]} para todos os nós")
        return root

    def connect_default_topology(self):
        """Reestabelece as conexões da topologia padrão do docker-compose.

        Garante um grafo conectado para que a propagação do bloco inicial
        alcance todos os nós durante o bootstrap.
        """
        edges = [
            ("node2", "node1"),
            ("node3", "node2"),
            ("node4", "node3"),
            ("node4", "node1"),
        ]
        for from_node, to_node in edges:
            if from_node in self.nodes and to_node in self.nodes:
                self.connect_if_needed(from_node, to_node)
        self.wait(1.0, "estabilização da topologia padrão")

    def peers(self, node: str) -> List[dict]:
        self._check_node(node)
        result = self.clients[node].call_or_raise("peers_list")
        return result.get("peers", []) if isinstance(result, dict) else []

    def mine(self, node: str) -> dict:
        self._check_node(node)
        log(f"Minerando bloco em {node}")
        started = time.time()
        result = self.clients[node].call_or_raise(
            "mine_block", timeout=self.mine_timeout
        )
        log(f"{node} concluiu mineração em {time.time() - started:.2f}s")
        return result

    def connect(self, from_node: str, to_node: str) -> dict:
        self._check_node(from_node)
        self._check_node(to_node)
        address = self.nodes[to_node].advertised_addr
        log(f"Conectando {from_node} -> {to_node} ({address})")
        result = self.clients[from_node].call_or_raise(
            "node_connect", {"address": address}
        )
        if isinstance(result, dict) and result.get("success") is False:
            raise RuntimeError(
                result.get("fail_message") or "node_connect retornou falha"
            )
        return result

    def connect_if_needed(self, from_node: str, to_node: str) -> Optional[dict]:
        target = self.nodes[to_node].advertised_addr
        if self._find_peer_by_advertised_addr(from_node, target):
            log(f"{from_node} já está conectado a {to_node}; mantendo conexão")
            return None
        return self.connect(from_node, to_node)

    def disconnect(self, from_node: str, to_node: str) -> dict:
        self._check_node(from_node)
        self._check_node(to_node)
        target = self.nodes[to_node].advertised_addr
        peer = self._find_peer_by_advertised_addr(from_node, target)
        if not peer:
            known = ", ".join(
                f"{p.get('advertised_addr') or '?'} via {p.get('addr')}"
                for p in self.peers(from_node)
            )
            raise RuntimeError(
                f"{from_node} não possui peer anunciado como {target}. Peers: {known or 'nenhum'}"
            )

        addr = peer["addr"]
        log(f"Desconectando {from_node} -> {to_node} usando addr real {addr}")
        result = self.clients[from_node].call_or_raise(
            "peer_disconnect", {"addr": addr}
        )
        if isinstance(result, dict) and result.get("success") is False:
            raise RuntimeError(
                result.get("message") or "peer_disconnect retornou falha"
            )
        return result

    def snapshot(self, show_peers: bool = True) -> List[dict]:
        rows = []
        for name in sorted(self.nodes):
            try:
                status = self.status(name)
                peers = self.peers(name) if show_peers else []
                rows.append(
                    {"node": name, "status": status, "peers": peers, "ok": True}
                )
            except Exception as e:
                rows.append({"node": name, "error": str(e), "ok": False})

        self.print_snapshot(rows, show_peers=show_peers)
        return rows

    def print_snapshot(self, rows: List[dict], show_peers: bool = True):
        print("\nSNAPSHOT DA REDE")
        print("-" * 80)
        for row in rows:
            name = row["node"]
            if not row["ok"]:
                print(f"{name:<5} ERRO: {row['error']}")
                continue

            status = row["status"]
            top_hash = str(status.get("top_block_hash", ""))[:16]
            print(
                f"{name:<5} height={status.get('block_height'):<4} "
                f"hash={top_hash} peers={status.get('peers_connected')} "
                f"addr={status.get('advertised_addr')}"
            )
            if show_peers:
                for peer in row["peers"]:
                    advertised = peer.get("advertised_addr") or "?"
                    print(
                        f"      peer={advertised:<12} real={peer.get('addr'):<22} "
                        f"dir={peer.get('direction'):<8} state={peer.get('handshake_state')}"
                    )
        print("-" * 80)

    def assert_converged(self) -> Tuple[int, str]:
        return self.assert_same_head(sorted(self.nodes), label="rede")

    def wait_until_converged(
        self, timeout_seconds: float = 30.0, interval: float = 1.0
    ) -> Tuple[int, str]:
        deadline = time.time() + timeout_seconds
        last_error: Optional[AssertionError] = None
        while time.time() < deadline:
            try:
                return self.assert_converged()
            except AssertionError as e:
                last_error = e
                time.sleep(interval)

        if last_error:
            raise last_error
        raise AssertionError("Rede não convergiu dentro do tempo esperado")

    def assert_same_head(
        self, nodes: List[str], label: Optional[str] = None
    ) -> Tuple[int, str]:
        for node in nodes:
            self._check_node(node)
        statuses = {name: self.status(name) for name in nodes}
        heads = {
            (status.get("block_height"), status.get("top_block_hash"))
            for status in statuses.values()
        }
        if len(heads) != 1:
            details = ", ".join(
                f"{name}=h{status.get('block_height')}:{str(status.get('top_block_hash'))[:12]}"
                for name, status in sorted(statuses.items())
            )
            group = label or ", ".join(nodes)
            raise AssertionError(f"{group} não está convergido: {details}")

        height, top_hash = next(iter(heads))
        group = label or ", ".join(nodes)
        log(f"OK: {group} convergiu em height={height}, hash={str(top_hash)[:16]}")
        return height, top_hash

    def assert_different_heads(
        self, left_nodes: List[str], right_nodes: List[str]
    ) -> Tuple[Tuple[int, str], Tuple[int, str]]:
        left = self.assert_same_head(left_nodes, label=", ".join(left_nodes))
        right = self.assert_same_head(right_nodes, label=", ".join(right_nodes))
        if left == right:
            raise AssertionError(
                "Os grupos deveriam estar em forks diferentes, mas compartilham o mesmo topo"
            )
        log(
            "OK: grupos estão em forks diferentes "
            f"(A=h{left[0]}:{str(left[1])[:12]}, B=h{right[0]}:{str(right[1])[:12]})"
        )
        return left, right

    def assert_diverged(self):
        statuses = {name: self.status(name) for name in self.nodes}
        heads = {
            (status.get("block_height"), status.get("top_block_hash"))
            for status in statuses.values()
        }
        if len(heads) <= 1:
            raise AssertionError("Rede não divergiu; todos os nós estão no mesmo topo")
        log(f"OK: rede divergiu temporariamente em {len(heads)} topos diferentes")

    def assert_peer_count(self, node: str, expected: int):
        actual = len(self.peers(node))
        if actual != expected:
            raise AssertionError(
                f"{node} deveria ter {expected} peers, mas possui {actual}"
            )
        log(f"OK: {node} possui {expected} peers")

    def disconnect_if_connected(self, from_node: str, to_node: str) -> Optional[dict]:
        target = self.nodes[to_node].advertised_addr
        if not self._find_peer_by_advertised_addr(from_node, target):
            log(f"{from_node} não está conectado a {to_node}; nada a desconectar")
            return None
        return self.disconnect(from_node, to_node)

    def _find_peer_by_advertised_addr(
        self, node: str, advertised_addr: str
    ) -> Optional[dict]:
        for peer in self.peers(node):
            if peer.get("advertised_addr") == advertised_addr:
                return peer
        return None

    def _check_node(self, node: str):
        if node not in self.nodes:
            raise ValueError(
                f"Nó desconhecido: {node}. Opções: {', '.join(sorted(self.nodes))}"
            )


def run_smoke(runner: ForkScenarioRunner):
    runner.title("Smoke test da infraestrutura de cenários")
    runner.step("Verificar se todos os nós estão online")
    runner.wait_until_up()
    runner.step("Garantir carteiras de minerador")
    runner.ensure_wallets()
    runner.step("Mostrar snapshot da rede")
    runner.snapshot()


def parse_args():
    parser = argparse.ArgumentParser(
        description="Runner base para cenários demonstráveis de fork do Caramuru"
    )
    parser.add_argument(
        "--pause",
        action="store_true",
        help="pausa antes de cada passo e continua quando Enter for pressionado",
    )
    parser.add_argument(
        "--step-delay",
        type=float,
        default=0.0,
        help="pausa automática entre passos quando --pause não está ativo",
    )
    parser.add_argument("--rpc-timeout", type=int, default=20)
    parser.add_argument("--mine-timeout", type=int, default=180)

    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("smoke", help="verifica conectividade e imprime snapshot")
    sub.add_parser("wait-up", help="aguarda todos os nós responderem")
    sub.add_parser(
        "ensure-wallets", help="cria/importa a carteira de minerador em todos os nós"
    )
    sub.add_parser("snapshot", help="imprime status e peers de todos os nós")
    sub.add_parser("assert-converged", help="falha se os nós não estiverem convergidos")

    status = sub.add_parser("status", help="mostra status bruto de um nó")
    status.add_argument("node", choices=sorted(NODES))

    mine = sub.add_parser("mine", help="minera um bloco em um nó")
    mine.add_argument("node", choices=sorted(NODES))

    connect = sub.add_parser("connect", help="conecta um nó a outro")
    connect.add_argument("from_node", choices=sorted(NODES))
    connect.add_argument("to_node", choices=sorted(NODES))

    disconnect = sub.add_parser("disconnect", help="desconecta um nó de outro")
    disconnect.add_argument("from_node", choices=sorted(NODES))
    disconnect.add_argument("to_node", choices=sorted(NODES))

    peers = sub.add_parser("peers", help="lista peers de um nó")
    peers.add_argument("node", choices=sorted(NODES))

    return parser.parse_args()


def main():
    args = parse_args()
    runner = ForkScenarioRunner(
        pause=args.pause,
        step_delay=args.step_delay,
        rpc_timeout=args.rpc_timeout,
        mine_timeout=args.mine_timeout,
    )

    if args.command == "smoke":
        run_smoke(runner)
    elif args.command == "wait-up":
        runner.wait_until_up()
    elif args.command == "ensure-wallets":
        runner.ensure_wallets()
    elif args.command == "snapshot":
        runner.snapshot()
    elif args.command == "assert-converged":
        runner.assert_converged()
    elif args.command == "status":
        print(json.dumps(runner.status(args.node), indent=2, ensure_ascii=False))
    elif args.command == "mine":
        runner.ensure_wallet(args.node)
        runner.mine(args.node)
        runner.snapshot(show_peers=False)
    elif args.command == "connect":
        runner.connect(args.from_node, args.to_node)
        runner.snapshot()
    elif args.command == "disconnect":
        runner.disconnect(args.from_node, args.to_node)
        runner.snapshot()
    elif args.command == "peers":
        print(json.dumps(runner.peers(args.node), indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
