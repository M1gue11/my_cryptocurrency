"""Cenário demonstrativo: nó atrasado se recupera após reconexão.

Este cenário ainda não força um fork concorrente. Ele valida a infraestrutura e
mostra uma propriedade básica necessária para os próximos testes de fork: um nó
isolado fica para trás, reconecta na rede e alcança novamente a cadeia canônica.
"""

import argparse

from fork_scenario_runner import ForkScenarioRunner


def run(runner: ForkScenarioRunner, propagation_wait: float):
    runner.title("Cenário hipotético: nó atrasado recupera após reconexão")

    runner.step("Aguardar todos os nós ficarem online")
    runner.wait_until_up()

    runner.ensure_wallets()

    runner.step("Estado inicial da rede")
    runner.snapshot()
    runner.assert_converged()

    runner.step("Isolar node4 removendo todas as suas conexões conhecidas")
    runner.disconnect_if_connected("node4", "node1")
    runner.disconnect_if_connected("node4", "node2")
    runner.disconnect_if_connected("node4", "node3")
    runner.wait(1.0, "fechamento das conexões P2P")
    runner.snapshot()
    runner.assert_peer_count("node4", 0)

    runner.step("Minerar blocos no grupo principal enquanto node4 está isolado")
    runner.mine("node1")
    runner.wait(propagation_wait, "propagação do primeiro bloco no grupo principal")
    runner.mine("node2")
    runner.wait(propagation_wait, "propagação do segundo bloco no grupo principal")
    runner.snapshot()
    runner.assert_diverged()

    runner.step("Reconectar node4 ao grupo principal")
    runner.connect_if_needed("node4", "node1")
    runner.wait(propagation_wait * 2, "sincronização do node4 após reconexão")
    runner.snapshot()

    runner.step("Verificar convergência final")
    runner.assert_converged()

    runner.title("RESULTADO: OK - node4 recuperou a cadeia após reconectar")


def parse_args():
    parser = argparse.ArgumentParser(
        description="Cenário: nó isolado fica atrasado e recupera após reconexão"
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
