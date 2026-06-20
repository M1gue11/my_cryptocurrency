"""Utilitários compartilhados pelos scripts de simulação.

Mantém em um único lugar a configuração dos nós Docker, chamadas JSON-RPC e o
setup idempotente da carteira do minerador usada pelos testes.
"""

import json
import threading
import urllib.error
import urllib.request
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Dict, Optional, Tuple


@dataclass(frozen=True)
class NodeEndpoint:
    name: str
    rpc_url: str
    advertised_addr: str


NODES: Dict[str, NodeEndpoint] = {
    "node1": NodeEndpoint("node1", "http://127.0.0.1:7101/rpc", "node1:6000"),
    "node2": NodeEndpoint("node2", "http://127.0.0.1:7102/rpc", "node2:6000"),
    "node3": NodeEndpoint("node3", "http://127.0.0.1:7103/rpc", "node3:6000"),
    "node4": NodeEndpoint("node4", "http://127.0.0.1:7104/rpc", "node4:6000"),
}

NODE_RPC_URLS = {name: endpoint.rpc_url for name, endpoint in NODES.items()}

# Caminho da carteira de minerador.
# IMPORTANTE: as chamadas RPC de wallet passam por um sandbox que enraiza o
# path em WALLET_KEYS_DIR (/data/keys no compose) e rejeita paths absolutos.
# Por isso enviamos o nome relativo "miner_wallet.json"; o sandbox o resolve
# para /data/keys/miner_wallet.json, que é exatamente onde o minerador
# (MINER_WALLET_SEED_PATH no compose) procura a carteira.
# A senha deve bater com MINER_WALLET_PASSWORD no docker-compose.sim.yml.
MINER_KEY_PATH = "miner_wallet.json"
MINER_PASSWORD = "miner123"

_rpc_id = 0
_rpc_id_lock = threading.Lock()


def next_rpc_id() -> int:
    """Gera IDs JSON-RPC únicos mesmo com chamadas paralelas."""
    global _rpc_id
    with _rpc_id_lock:
        _rpc_id += 1
        return _rpc_id


def ts() -> str:
    return datetime.now(timezone.utc).strftime("%H:%M:%S")


def log(message: str):
    print(f"[{ts()}] {message}", flush=True)


def iso_now() -> str:
    return datetime.now(timezone.utc).isoformat()


class RpcError(RuntimeError):
    pass


class RpcClient:
    def __init__(self, url: str, timeout: int = 15):
        self.url = url
        self.timeout = timeout

    def call(
        self, method: str, params=None, timeout: Optional[int] = None
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
            with urllib.request.urlopen(req, timeout=timeout or self.timeout) as resp:
                body = json.loads(resp.read().decode("utf-8"))
        except urllib.error.URLError as e:
            return None, f"transport: {e}"
        except json.JSONDecodeError as e:
            return None, f"decode: {e}"
        if isinstance(body, dict) and body.get("error"):
            return None, body["error"]
        return (body.get("result") if isinstance(body, dict) else body), None

    def call_or_raise(self, method: str, params=None, timeout: Optional[int] = None):
        result, error = self.call(method, params=params, timeout=timeout)
        if error:
            raise RpcError(f"RPC error calling {method}: {error}")
        return result


def ensure_miner_wallet(
    rpc: RpcClient,
    node_name: str,
    wallet_path: str = MINER_KEY_PATH,
    wallet_password: str = MINER_PASSWORD,
) -> Tuple[Optional[str], Optional[str]]:
    """Cria ou importa a carteira do minerador de forma idempotente."""
    res, err = rpc.call(
        "wallet_new",
        {"password": wallet_password, "path": wallet_path},
    )
    if res and res.get("address"):
        address = res["address"]
        log(f"  {node_name}: carteira criada {address[:16]}...")
        return address, None

    res2, err2 = rpc.call(
        "wallet_import",
        {"password": wallet_password, "path": wallet_path},
    )
    if res2 and res2.get("address"):
        address = res2["address"]
        log(f"  {node_name}: carteira ja existia {address[:16]}...")
        return address, None

    error = str(err or err2 or "falha desconhecida ao criar/importar carteira")
    log(f"  {node_name}: FALHA ao criar/importar carteira: {error}")
    return None, error
