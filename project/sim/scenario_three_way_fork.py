"""Cenário demonstrativo: três branches concorrentes e uma vencedora.

A rede é separada em três visões: node1 sozinho, node2 sozinho e node3/node4
juntos. Cada lado cria uma branch concorrente; node3/node4 estendem a deles e,
após a reconexão, todos devem convergir para essa branch mais longa.
"""

import argparse

from fork_scenario_runner import ForkScenarioRunner

BRANCH_A = ["node1"]
BRANCH_B = ["node2"]
WINNING_BRANCH = ["node3", "node4"]


def isolate_three_branches(runner: ForkScenarioRunner):
    isolated_pairs = [
        ("node1", "node2"),
        ("node1", "node3"),
        ("node1", "node4"),
        ("node2", "node3"),
        ("node2", "node4"),
    ]
    for left, right in isolated_pairs:
        runner.disconnect_if_connected(left, right)
        runner.disconnect_if_connected(right, left)

    runner.wait(1.0, "fechamento das conexões entre branches")
    runner.connect_if_needed("node4", "node3")
    runner.wait(1.0, "garantia da branch node3/node4")


def run(runner: ForkScenarioRunner, propagation_wait: float):
    runner.title("Cenário: fork com três branches concorrentes")

    runner.step("Aguardar todos os nós ficarem online")
    runner.wait_until_up()
    runner.ensure_wallets()

    runner.step("Garantir bloco raiz comum antes de particionar a rede")
    runner.bootstrap_common_root()

    runner.step("Confirmar estado inicial convergido")
    runner.snapshot()
    runner.assert_converged()

    runner.step("Separar a rede em três branches: node1, node2 e node3/node4")
    isolate_three_branches(runner)
    runner.snapshot()
    runner.assert_peer_count("node1", 0)
    runner.assert_peer_count("node2", 0)
    runner.assert_same_head(WINNING_BRANCH, label="branch node3/node4")

    runner.step("Criar três blocos concorrentes na mesma altura")
    runner.mine("node1")
    runner.wait(propagation_wait, "branch isolada do node1")
    runner.mine("node2")
    runner.wait(propagation_wait, "branch isolada do node2")
    runner.mine("node3")
    runner.wait(propagation_wait, "primeiro bloco da branch node3/node4")

    branch_a_head = runner.assert_same_head(BRANCH_A, label="branch node1")
    branch_b_head = runner.assert_same_head(BRANCH_B, label="branch node2")
    winning_head = runner.assert_same_head(WINNING_BRANCH, label="branch node3/node4")
    if len({branch_a_head, branch_b_head, winning_head}) != 3:
        raise AssertionError("As três branches deveriam ter topos diferentes")
    if not (branch_a_head[0] == branch_b_head[0] == winning_head[0]):
        raise AssertionError(
            "As três branches deveriam começar empatadas na mesma altura"
        )

    runner.step("Estender a branch node3/node4 para ela vencer o fork")
    runner.mine("node4")
    runner.wait(propagation_wait, "segundo bloco da branch vencedora")
    winning_head = runner.assert_same_head(WINNING_BRANCH, label="branch vencedora")
    runner.snapshot()
    runner.assert_different_heads(BRANCH_A, WINNING_BRANCH)
    runner.assert_different_heads(BRANCH_B, WINNING_BRANCH)

    runner.step("Reconectar node1 e node2 à branch vencedora")
    runner.connect_if_needed("node1", "node3")
    runner.connect_if_needed("node2", "node4")
    runner.wait_until_converged(timeout_seconds=max(35.0, propagation_wait * 12))
    runner.snapshot()
    final_head = runner.assert_converged()
    if final_head != winning_head:
        raise AssertionError(
            "A rede convergiu, mas não para a branch vencedora esperada: "
            f"esperado h{winning_head[0]}:{str(winning_head[1])[:12]}, "
            f"obtido h{final_head[0]}:{str(final_head[1])[:12]}"
        )

    runner.title("RESULTADO: OK - três branches convergiram para a mais longa")


def parse_args():
    parser = argparse.ArgumentParser(
        description="Cenário: três branches concorrentes resolvidas pela branch mais longa"
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
