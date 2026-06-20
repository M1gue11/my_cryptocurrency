"""Cenário demonstrativo: reorg profundo para uma branch mais longa.

O grupo local minera uma branch com vários blocos, mas o grupo remoto produz uma
branch ainda mais longa durante a partição. Ao reconectar, os nós do grupo local
precisam abandonar múltiplos blocos e reorganizar para a branch vencedora.
"""

import argparse

from fork_scenario_runner import ForkScenarioRunner

LOCAL_GROUP = ["node1", "node2"]
REMOTE_GROUP = ["node3", "node4"]


def isolate_groups(runner: ForkScenarioRunner):
    for left in LOCAL_GROUP:
        for right in REMOTE_GROUP:
            runner.disconnect_if_connected(left, right)
            runner.disconnect_if_connected(right, left)

    runner.wait(1.0, "fechamento das conexões entre os grupos")
    runner.connect_if_needed("node2", "node1")
    runner.connect_if_needed("node4", "node3")
    runner.wait(1.0, "garantia das conexões internas de cada grupo")


def mine_sequence(
    runner: ForkScenarioRunner, nodes, propagation_wait: float, reason: str
):
    for index, node in enumerate(nodes, start=1):
        runner.mine(node)
        runner.wait(propagation_wait, f"{reason} ({index}/{len(nodes)})")


def run(runner: ForkScenarioRunner, propagation_wait: float):
    runner.title("Cenário: reorg profundo para branch mais longa")

    runner.step("Aguardar todos os nós ficarem online")
    runner.wait_until_up()
    runner.ensure_wallets()

    runner.step("Confirmar estado inicial convergido")
    runner.snapshot()
    runner.assert_converged()

    runner.step("Particionar a rede em grupo local e grupo remoto")
    isolate_groups(runner)
    runner.snapshot()
    runner.assert_same_head(LOCAL_GROUP, label="grupo local")
    runner.assert_same_head(REMOTE_GROUP, label="grupo remoto")

    runner.step("Minerar 2 blocos no grupo local que depois será reorganizado")
    mine_sequence(
        runner,
        ["node1", "node2"],
        propagation_wait,
        "propagação da branch local",
    )
    local_head = runner.assert_same_head(LOCAL_GROUP, label="grupo local")

    runner.step("Minerar 5 blocos no grupo remoto, criando a branch vencedora")
    mine_sequence(
        runner,
        ["node3", "node4", "node3", "node4", "node3"],
        propagation_wait,
        "propagação da branch remota",
    )
    remote_head = runner.assert_same_head(REMOTE_GROUP, label="grupo remoto")
    if remote_head[0] <= local_head[0]:
        raise AssertionError("A branch remota deveria ser mais longa que a local")

    runner.snapshot()
    runner.assert_different_heads(LOCAL_GROUP, REMOTE_GROUP)

    runner.step("Reconectar os grupos e forçar reorg profundo no grupo local")
    runner.connect_if_needed("node2", "node3")
    runner.wait_until_converged(timeout_seconds=max(40.0, propagation_wait * 14))
    runner.snapshot()
    final_head = runner.assert_converged()
    if final_head != remote_head:
        raise AssertionError(
            "A rede convergiu, mas não para a branch remota mais longa: "
            f"esperado h{remote_head[0]}:{str(remote_head[1])[:12]}, "
            f"obtido h{final_head[0]}:{str(final_head[1])[:12]}"
        )

    runner.title("RESULTADO: OK - reorg profundo adotou a branch mais longa")


def parse_args():
    parser = argparse.ArgumentParser(
        description="Cenário: reorg profundo para branch mais longa"
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
    parser.add_argument(
        "--propagation-wait",
        type=float,
        default=2.0,
        help="tempo de espera para propagação/sincronização entre ações",
    )
    return parser.parse_args()


def main():
    args = parse_args()
    runner = ForkScenarioRunner(
        pause=args.pause,
        step_delay=args.step_delay,
        rpc_timeout=args.rpc_timeout,
        mine_timeout=args.mine_timeout,
    )
    run(runner, propagation_wait=args.propagation_wait)


if __name__ == "__main__":
    main()
