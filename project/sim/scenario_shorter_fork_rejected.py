"""Cenário demonstrativo: fork curto tardio é rejeitado.

Um nó isolado minera sua própria branch enquanto a rede principal avança mais.
Ao reconectar, a branch curta do nó isolado deve ser abandonada em favor da
cadeia principal mais longa.
"""

import argparse

from fork_scenario_runner import ForkScenarioRunner

MAIN_GROUP = ["node1", "node2", "node3"]
ISOLATED_NODE = "node4"


def isolate_node4(runner: ForkScenarioRunner):
    for node in MAIN_GROUP:
        runner.disconnect_if_connected(ISOLATED_NODE, node)
        runner.disconnect_if_connected(node, ISOLATED_NODE)

    runner.wait(1.0, "fechamento das conexões do node4")
    runner.connect_if_needed("node2", "node1")
    runner.connect_if_needed("node3", "node2")
    runner.wait(1.0, "garantia da topologia interna da rede principal")


def mine_sequence(
    runner: ForkScenarioRunner, nodes, propagation_wait: float, reason: str
):
    for index, node in enumerate(nodes, start=1):
        runner.mine(node)
        runner.wait(propagation_wait, f"{reason} ({index}/{len(nodes)})")


def run(runner: ForkScenarioRunner, propagation_wait: float):
    runner.title("Cenário: fork curto tardio é rejeitado")

    runner.step("Aguardar todos os nós ficarem online")
    runner.wait_until_up()
    runner.ensure_wallets()

    runner.step("Confirmar estado inicial convergido")
    runner.snapshot()
    runner.assert_converged()

    runner.step("Isolar node4 e manter node1/node2/node3 como rede principal")
    isolate_node4(runner)
    runner.snapshot()
    runner.assert_peer_count(ISOLATED_NODE, 0)
    runner.assert_same_head(MAIN_GROUP, label="rede principal")

    runner.step("Minerar 4 blocos na rede principal")
    mine_sequence(
        runner,
        ["node1", "node2", "node3", "node1"],
        propagation_wait,
        "propagação da cadeia principal",
    )
    main_head = runner.assert_same_head(MAIN_GROUP, label="rede principal")

    runner.step("Minerar 2 blocos conflitantes no node4 isolado")
    mine_sequence(
        runner,
        [ISOLATED_NODE, ISOLATED_NODE],
        propagation_wait,
        "mineração isolada do fork curto",
    )
    isolated_head = runner.assert_same_head([ISOLATED_NODE], label="node4 isolado")
    if isolated_head[0] >= main_head[0]:
        raise AssertionError("O fork do node4 deveria ser menor que a cadeia principal")

    runner.snapshot()
    runner.assert_different_heads(MAIN_GROUP, [ISOLATED_NODE])

    runner.step("Reconectar node4 e verificar abandono do fork curto")
    runner.connect_if_needed(ISOLATED_NODE, "node1")
    runner.wait_until_converged(timeout_seconds=max(30.0, propagation_wait * 10))
    runner.snapshot()
    final_head = runner.assert_converged()
    if final_head != main_head:
        raise AssertionError(
            "A rede convergiu, mas não para a cadeia principal esperada: "
            f"esperado h{main_head[0]}:{str(main_head[1])[:12]}, "
            f"obtido h{final_head[0]}:{str(final_head[1])[:12]}"
        )

    runner.title("RESULTADO: OK - o fork curto do node4 foi rejeitado")


def parse_args():
    parser = argparse.ArgumentParser(
        description="Cenário: nó isolado retorna com fork menor e abandona sua branch"
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
