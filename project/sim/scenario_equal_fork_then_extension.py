"""Cenário demonstrativo: fork empatado resolvido após extensão.

O roteiro particiona a rede em dois grupos, faz cada grupo minerar um bloco na
mesma altura e reconecta os grupos. Como o fork está empatado, a demonstração
não depende de escolher um vencedor imediatamente. Em seguida, uma das branches
é estendida e a rede deve convergir para ela.
"""

import argparse

from fork_scenario_runner import ForkScenarioRunner

GROUP_A = ["node1", "node2"]
GROUP_B = ["node3", "node4"]


def isolate_groups(runner: ForkScenarioRunner):
    for left in GROUP_A:
        for right in GROUP_B:
            runner.disconnect_if_connected(left, right)
            runner.disconnect_if_connected(right, left)

    runner.wait(1.0, "fechamento das conexões entre os grupos")
    runner.connect_if_needed("node2", "node1")
    runner.connect_if_needed("node4", "node3")
    runner.wait(1.0, "garantia das conexões internas de cada grupo")


def run(runner: ForkScenarioRunner, propagation_wait: float):
    runner.title("Cenário: fork empatado resolvido após extensão")

    runner.step("Aguardar todos os nós ficarem online")
    runner.wait_until_up()
    runner.ensure_wallets()

    runner.step("Garantir bloco raiz comum antes de particionar a rede")
    runner.bootstrap_common_root()

    runner.step("Confirmar estado inicial convergido")
    runner.snapshot()
    runner.assert_converged()

    runner.step("Particionar a rede em dois grupos: node1/node2 e node3/node4")
    isolate_groups(runner)
    runner.snapshot()
    runner.assert_same_head(GROUP_A, label="grupo A")
    runner.assert_same_head(GROUP_B, label="grupo B")

    runner.step("Minerar um bloco em cada grupo, criando fork na mesma altura")
    runner.mine("node1")
    runner.wait(propagation_wait, "propagação interna no grupo A")
    runner.mine("node3")
    runner.wait(propagation_wait, "propagação interna no grupo B")
    runner.snapshot()
    group_a_head, group_b_head = runner.assert_different_heads(GROUP_A, GROUP_B)
    if group_a_head[0] != group_b_head[0]:
        raise AssertionError(
            "Esperava fork empatado na mesma altura, mas os grupos ficaram em alturas diferentes"
        )

    runner.step("Reconectar os grupos e observar que o empate ainda é um fork")
    runner.connect_if_needed("node2", "node3")
    runner.wait(propagation_wait * 2, "troca de blocos concorrentes entre os grupos")
    runner.snapshot()
    runner.assert_different_heads(GROUP_A, GROUP_B)

    runner.step("Estender a branch do grupo A para desempatar o fork")
    runner.mine("node1")
    runner.wait_until_converged(timeout_seconds=max(20.0, propagation_wait * 6))
    runner.snapshot()
    final_height, _ = runner.assert_converged()
    if final_height != group_a_head[0] + 1:
        raise AssertionError(
            f"Altura final esperada era {group_a_head[0] + 1}, mas foi {final_height}"
        )

    runner.title(
        "RESULTADO: OK - fork empatado convergiu após uma branch ser estendida"
    )


def parse_args():
    parser = argparse.ArgumentParser(
        description="Cenário: dois grupos criam fork empatado e convergem após extensão"
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
