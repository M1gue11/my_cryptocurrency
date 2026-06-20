"""Cenário demonstrativo: a branch mais longa vence após reconexão.

O roteiro particiona a rede em dois grupos. O grupo A minera uma cadeia curta,
enquanto o grupo B minera uma cadeia mais longa. Ao reconectar os grupos, todos
os nós devem convergir para o topo da branch mais longa.
"""

import argparse

from fork_scenario_runner import ForkScenarioRunner

SHORT_GROUP = ["node1", "node2"]
LONG_GROUP = ["node3", "node4"]


def isolate_groups(runner: ForkScenarioRunner):
    for left in SHORT_GROUP:
        for right in LONG_GROUP:
            runner.disconnect_if_connected(left, right)
            runner.disconnect_if_connected(right, left)

    runner.wait(1.0, "fechamento das conexões entre os grupos")
    runner.connect_if_needed("node2", "node1")
    runner.connect_if_needed("node4", "node3")
    runner.wait(1.0, "garantia das conexões internas de cada grupo")


def run(runner: ForkScenarioRunner, propagation_wait: float):
    runner.title("Cenário: branch mais longa vence o fork")

    runner.step("Aguardar todos os nós ficarem online")
    runner.wait_until_up()
    runner.ensure_wallets()

    runner.step("Confirmar estado inicial convergido")
    runner.snapshot()
    runner.assert_converged()

    runner.step("Particionar a rede em grupo lento e grupo rapido")
    isolate_groups(runner)
    runner.snapshot()
    runner.assert_same_head(SHORT_GROUP, label="grupo lento")
    runner.assert_same_head(LONG_GROUP, label="grupo rapido")

    runner.step("Minerar apenas um bloco no grupo lento")
    runner.mine("node1")
    runner.wait(propagation_wait, "propagação interna no grupo lento")
    short_head = runner.assert_same_head(SHORT_GROUP, label="grupo lento")

    runner.step("Minerar dois blocos no grupo rapido")
    runner.mine("node3")
    runner.wait(propagation_wait, "propagação do primeiro bloco no grupo rapido")
    runner.mine("node4")
    runner.wait(propagation_wait, "propagação do segundo bloco no grupo rapido")
    long_head = runner.assert_same_head(LONG_GROUP, label="grupo rapido")
    if long_head[0] <= short_head[0]:
        raise AssertionError(
            "O grupo rapido deveria estar em altura maior que o grupo lento"
        )

    runner.snapshot()
    runner.assert_different_heads(SHORT_GROUP, LONG_GROUP)

    runner.step("Reconectar os grupos e aguardar a reorganização para a branch longa")
    runner.connect_if_needed("node2", "node3")
    runner.wait_until_converged(timeout_seconds=max(20.0, propagation_wait * 8))
    runner.snapshot()
    final_head = runner.assert_converged()
    if final_head != long_head:
        raise AssertionError(
            "A rede convergiu, mas não para a branch mais longa esperada: "
            f"esperado h{long_head[0]}:{str(long_head[1])[:12]}, "
            f"obtido h{final_head[0]}:{str(final_head[1])[:12]}"
        )

    runner.title("RESULTADO: OK - todos os nós adotaram a branch mais longa")


def parse_args():
    parser = argparse.ArgumentParser(
        description="Cenário: fork entre cadeia curta e cadeia longa"
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
        default=3.0,
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
