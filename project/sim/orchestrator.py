#!/usr/bin/env python3
"""
Orquestrador do experimento distribuido do Caramuru - Nivel 3 (4 nos).

Fala JSON-RPC 2.0 via HTTP (POST /rpc) com cada no exposto pelo
docker-compose.sim.yml. Executa uma sequencia de acoes pseudoaleatorias
controladas por SEED FIXA, para que o experimento seja reproduzivel, e
coleta metricas de altura/top-hash/peers ao longo do tempo para avaliar
sincronizacao, propagacao e convergencia entre os nos.

Uso tipico:
    docker compose -f docker-compose.sim.yml up --build -d
    python3 sim/orchestrator.py --rounds 30 --seed 42
    docker compose -f docker-compose.sim.yml down -v

Sem dependencias externas: usa apenas a biblioteca padrao.
"""
import argparse
import json
import random
import sys
import time
import urllib.request
import urllib.error
from datetime import datetime, timezone

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
MINER_KEY_PATH = "miner_wallet.json"
MINER_PASSWORD = "miner123"

_rpc_id = 0


def rpc(url, method, params=None, timeout=15):
    """Faz uma chamada JSON-RPC 2.0 e devolve (result, error)."""
    global _rpc_id
    _rpc_id += 1
    payload = {"jsonrpc": "2.0", "id": _rpc_id, "method": method}
    if params is not None:
        payload["params"] = params
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        url, data=data, headers={"Content-Type": "application/json"}
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


def ts():
    return datetime.now(timezone.utc).strftime("%H:%M:%S")


def log(msg):
    print(f"[{ts()}] {msg}", flush=True)


def wait_until_up(retries=30, delay=2):
    """Espera todos os nos responderem node_status."""
    log("Aguardando os 4 nos ficarem prontos...")
    for attempt in range(retries):
        ready = []
        for name, url in NODES.items():
            res, err = rpc(url, "node_status")
            if res is not None:
                ready.append(name)
        if len(ready) == len(NODES):
            log(f"Todos prontos: {', '.join(ready)}")
            return True
        log(f"  tentativa {attempt + 1}/{retries}: prontos={ready}")
        time.sleep(delay)
    return False


def ensure_wallets():
    """Cria a carteira de minerador em cada no (idempotente)."""
    log("Garantindo carteira de minerador em cada no...")
    addresses = {}
    for name, url in NODES.items():
        res, err = rpc(
            url,
            "wallet_new",
            {"password": MINER_PASSWORD, "path": MINER_KEY_PATH},
        )
        if res and res.get("address"):
            addresses[name] = res["address"]
            log(f"  {name}: carteira criada {res['address'][:16]}...")
        else:
            # Ja existe -> importa para recuperar o endereco.
            res2, err2 = rpc(
                url,
                "wallet_import",
                {"password": MINER_PASSWORD, "path": MINER_KEY_PATH},
            )
            if res2 and res2.get("address"):
                addresses[name] = res2["address"]
                log(f"  {name}: carteira ja existia {res2['address'][:16]}...")
            else:
                log(f"  {name}: FALHA ao criar/importar carteira: {err or err2}")
    return addresses


def snapshot():
    """Coleta altura, top-hash e peers de cada no."""
    snap = {}
    for name, url in NODES.items():
        res, err = rpc(url, "node_status")
        if res is None:
            snap[name] = {"error": str(err)}
        else:
            snap[name] = {
                "height": res.get("block_height"),
                "top": (res.get("top_block_hash") or "")[:16],
                "peers": res.get("peers_connected"),
            }
    return snap


def is_converged(snap):
    """True se todos os nos tem a mesma altura e mesmo top-hash."""
    tops = set()
    heights = set()
    for v in snap.values():
        if "error" in v:
            return False
        tops.add(v["top"])
        heights.add(v["height"])
    return len(tops) == 1 and len(heights) == 1


def print_snapshot(snap):
    parts = []
    for name in NODES:
        v = snap[name]
        if "error" in v:
            parts.append(f"{name}=ERR")
        else:
            parts.append(f"{name}=h{v['height']}/{v['top']}/p{v['peers']}")
    flag = "CONVERGIDO" if is_converged(snap) else "divergente"
    log(f"  estado: {' | '.join(parts)}  [{flag}]")


def action_mine(rng, addresses):
    """Minera 1 bloco em um no aleatorio."""
    name = rng.choice(list(NODES))
    res, err = rpc(NODES[name], "mine_block", timeout=120)
    if err:
        log(f"  acao=mine no={name} -> ERRO: {err}")
        return {"action": "mine", "node": name, "ok": False, "error": str(err)}
    blk = res.get("block", {}) if isinstance(res, dict) else {}
    log(f"  acao=mine no={name} -> ok (resp recebida)")
    return {"action": "mine", "node": name, "ok": True}


def action_send(rng, addresses):
    """Envia uma transacao de um no para o endereco de outro no."""
    src = rng.choice(list(NODES))
    dst = rng.choice([n for n in NODES if n != src])
    to_addr = addresses.get(dst)
    if not to_addr:
        return {"action": "send", "ok": False, "error": "sem endereco destino"}
    amount = rng.randint(1, 5) * 100000  # fracoes de COIN (COIN=1_000_000)
    params = {
        "from": {"key_path": MINER_KEY_PATH, "password": MINER_PASSWORD},
        "to": to_addr,
        "amount": amount,
        "fee": 10000,
        "message": "sim-tx",
    }
    res, err = rpc(NODES[src], "wallet_send", params)
    ok = bool(res and res.get("success"))
    detail = (res or {}).get("error") if not ok else (res or {}).get("tx_id", "")
    log(f"  acao=send {src}->{dst} amt={amount} -> {'ok' if ok else 'falha'}: {str(detail)[:40]}")
    return {"action": "send", "src": src, "dst": dst, "amount": amount, "ok": ok}


def action_query(rng, addresses):
    """Consulta mempool de um no aleatorio (acao observacional leve)."""
    name = rng.choice(list(NODES))
    res, err = rpc(NODES[name], "node_mempool")
    count = (res or {}).get("count") if res else None
    log(f"  acao=query no={name} -> mempool count={count}")
    return {"action": "query", "node": name, "mempool": count, "ok": err is None}


ACTIONS = [
    (action_mine, 0.55),
    (action_send, 0.30),
    (action_query, 0.15),
]


def pick_action(rng):
    r = rng.random()
    acc = 0.0
    for fn, w in ACTIONS:
        acc += w
        if r <= acc:
            return fn
    return ACTIONS[-1][0]


def main():
    ap = argparse.ArgumentParser(description="Orquestrador da simulacao distribuida do Caramuru")
    ap.add_argument("--rounds", type=int, default=30, help="numero de rodadas de acao")
    ap.add_argument("--seed", type=int, default=42, help="seed fixa para reprodutibilidade")
    ap.add_argument("--delay", type=float, default=1.5, help="pausa (s) entre rodadas")
    ap.add_argument("--settle", type=float, default=8.0, help="espera final (s) para convergencia")
    ap.add_argument("--out", default="sim/results.json", help="arquivo de saida das metricas")
    args = ap.parse_args()

    rng = random.Random(args.seed)
    log(f"=== Simulacao distribuida Caramuru | seed={args.seed} rounds={args.rounds} ===")

    if not wait_until_up():
        log("ERRO: nem todos os nos ficaram prontos. Abortando.")
        sys.exit(1)

    addresses = ensure_wallets()
    time.sleep(2)

    history = []
    log("Estado inicial:")
    snap0 = snapshot()
    print_snapshot(snap0)
    history.append({"round": 0, "event": "initial", "snapshot": snap0})

    for i in range(1, args.rounds + 1):
        fn = pick_action(rng)
        log(f"Rodada {i}/{args.rounds}:")
        result = fn(rng, addresses)
        time.sleep(args.delay)
        snap = snapshot()
        print_snapshot(snap)
        history.append({"round": i, "event": result, "snapshot": snap})

    log(f"Aguardando {args.settle}s para convergencia final...")
    time.sleep(args.settle)
    snap_final = snapshot()
    log("Estado final:")
    print_snapshot(snap_final)
    converged = is_converged(snap_final)
    history.append({"round": args.rounds + 1, "event": "final", "snapshot": snap_final})

    summary = {
        "seed": args.seed,
        "rounds": args.rounds,
        "converged_final": converged,
        "final_snapshot": snap_final,
        "generated_at": datetime.now(timezone.utc).isoformat(),
    }
    report = {"summary": summary, "history": history}
    with open(args.out, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2, ensure_ascii=False)

    log(f"Relatorio salvo em {args.out}")
    log(f"Convergencia final: {'SIM' if converged else 'NAO'}")
    sys.exit(0 if converged else 2)


if __name__ == "__main__":
    main()
