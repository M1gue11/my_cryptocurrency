"""Cenário demonstrativo: resolução de fork por nó ponte.

A rede é particionada em dois grupos e cada grupo avança em uma branch. Depois,
a reconexão é feita por uma única aresta node2 -> node3, sem restaurar a malha
completa. O objetivo é mostrar que uma topologia parcial ainda propaga a branch
vencedora até toda a rede.
"""

import argparse

from fork_scenario_runner import ForkScenarioRunner

LEFT_GROUP = ["node1", "node2"]
RIGHT_GROUP = ["node3", "node4"]


def isolate_groups(runner: ForkScenarioRunner):
    for left in LEFT_GROUP:
        for right in RIGHT_GROUP:
            runner.disconnect_if_connected(left, right)
            runner.disconnect_if_connected(right, left)

    runner.wait(1.0, "fechamento das conexões entre os grupos")
    runner.connect_if_needed("node2", "node1")
    runner.connect_if_needed("node4", "node3")
    runner.wait(1.0, "garantia das conexões internas de cada lado")


def mine_sequence(
    runner: ForkScenarioRunner, nodes, propagation_wait: float, reason: str
):
    for index, node in enumerate(nodes, start=1):
        runner.mine(node)
        runner.wait(propagation_wait, f"{reason} ({index}/{len(nodes)})")


def run(runner: ForkScenarioRunner, propagation_wait: float):
    runner.title("Cenário: fork resolvido por nó ponte")

    runner.step("Aguardar todos os nós ficarem online")
    runner.wait_until_up()
    runner.ensure_wallets()

    runner.step("Garantir bloco raiz comum antes de particionar a rede")
    runner.bootstrap_common_root()

    runner.step("Confirmar estado inicial convergido")
    runner.snapshot()
    runner.assert_converged()

    runner.step("Particionar a rede em dois lados conectados internamente")
    isolate_groups(runner)
    runner.snapshot()
    runner.assert_same_head(LEFT_GROUP, label="lado esquerdo")
    runner.assert_same_head(RIGHT_GROUP, label="lado direito")

    runner.step("Minerar 2 blocos no lado esquerdo")
    mine_sequence(
        runner,
        ["node1", "node2"],
        propagation_wait,
        "propagação no lado esquerdo",
    )
    left_head = runner.assert_same_head(LEFT_GROUP, label="lado esquerdo")

    runner.step("Minerar 4 blocos no lado direito, que será a branch vencedora")
    mine_sequence(
        runner,
        ["node3", "node4", "node3", "node4"],
        propagation_wait,
        "propagação no lado direito",
    )
    right_head = runner.assert_same_head(RIGHT_GROUP, label="lado direito")
    if right_head[0] <= left_head[0]:
        raise AssertionError("O lado direito deveria estar em altura maior")

    runner.snapshot()
    runner.assert_different_heads(LEFT_GROUP, RIGHT_GROUP)

    runner.step("Reconectar apenas node2 -> node3 como ponte entre os grupos")
    runner.connect_if_needed("node2", "node3")
    runner.wait(propagation_wait * 2, "propagação inicial pela ponte node2/node3")
    runner.snapshot()

    runner.step("Aguardar a branch vencedora atravessar a ponte e alcançar todos")
    runner.wait_until_converged(timeout_seconds=max(40.0, propagation_wait * 14))
    runner.snapshot()
    final_head = runner.assert_converged()
    if final_head != right_head:
        raise AssertionError(
            "A rede convergiu, mas não para a branch esperada do lado direito: "
            f"esperado h{right_head[0]}:{str(right_head[1])[:12]}, "
            f"obtido h{final_head[0]}:{str(final_head[1])[:12]}"
        )

    runner.title("RESULTADO: OK - topologia parcial com nó ponte resolveu o fork")


def parse_args():
    parser = argparse.ArgumentParser(
        description="Cenário: fork resolvido por uma única conexão ponte"
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
