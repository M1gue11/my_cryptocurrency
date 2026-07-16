"""Executa os cenários de fork como uma suíte de testes.

Roda cada `scenario_*.py` em modo não-interativo (sem `--pause`), reportando
sucesso/falha e duração de cada um, e sai com código != 0 se qualquer cenário
falhar. Por padrão o ambiente é resetado antes de cada cenário (volumes Docker
recriados), garantindo isolamento estilo "teste unitário": cada cenário parte de
uma rede limpa e fixa seu próprio bloco raiz comum.

Uso (a partir de project/):
    python .\\sim\\run_scenarios.py                # roda todos, resetando entre cada
    python .\\sim\\run_scenarios.py --no-isolate   # roda todos em sequência, sem reset
    python .\\sim\\run_scenarios.py --build         # força rebuild da imagem
    python .\\sim\\run_scenarios.py deep_reorg late_rejoin   # subconjunto por nome
"""

import argparse
import os
import subprocess
import sys
import time

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_DIR = os.path.dirname(SCRIPT_DIR)
COMPOSE_FILE = os.path.join(PROJECT_DIR, "docker-compose.sim.yml")

# Ordem determinística da suíte. Os nomes batem com os arquivos
# scenario_<nome>.py neste diretório.
SCENARIOS = [
    "late_rejoin",
    "equal_fork_then_extension",
    "deep_reorg",
    "shorter_fork_rejected",
    "three_way_fork",
    "staged_healing_bridge",
]


def scenario_path(name: str) -> str:
    return os.path.join(SCRIPT_DIR, f"scenario_{name}.py")


def run_compose(args, check=True):
    cmd = ["docker", "compose", "-f", COMPOSE_FILE, *args]
    print(f"$ {' '.join(cmd)}", flush=True)
    result = subprocess.run(cmd, cwd=PROJECT_DIR)
    if check and result.returncode != 0:
        raise RuntimeError(
            f"Comando docker compose falhou ({result.returncode}): {' '.join(args)}"
        )
    return result.returncode


def reset_environment(build: bool):
    """Derruba os volumes e sobe a rede novamente, do zero."""
    run_compose(["down", "-v"], check=False)
    up_args = ["up", "-d"]
    if build:
        up_args.insert(1, "--build")
    run_compose(up_args)


def run_scenario(name: str) -> int:
    path = scenario_path(name)
    if not os.path.exists(path):
        print(f"[SKIP] cenário inexistente: {path}", flush=True)
        return 127
    print("\n" + "#" * 80, flush=True)
    print(f"# Executando cenário: {name}", flush=True)
    print("#" * 80, flush=True)
    result = subprocess.run([sys.executable, path], cwd=PROJECT_DIR)
    return result.returncode


def parse_args():
    parser = argparse.ArgumentParser(
        description="Suíte de testes dos cenários de fork do Caramuru"
    )
    parser.add_argument(
        "scenarios",
        nargs="*",
        help="subconjunto de cenários a rodar (por nome, sem o prefixo scenario_). "
        "Se omitido, roda todos.",
    )
    parser.add_argument(
        "--no-isolate",
        dest="isolate",
        action="store_false",
        help="não reseta o ambiente entre cenários (mais rápido, sem isolamento)",
    )
    parser.add_argument(
        "--build",
        action="store_true",
        help="força rebuild da imagem Docker ao (re)subir a rede",
    )
    parser.add_argument(
        "--keep-up",
        action="store_true",
        help="mantém a rede no ar ao final (não roda 'down -v')",
    )
    return parser.parse_args()


def main():
    args = parse_args()

    selected = args.scenarios or SCENARIOS
    unknown = [s for s in selected if s not in SCENARIOS]
    if unknown:
        print(
            f"Cenários desconhecidos: {', '.join(unknown)}. "
            f"Disponíveis: {', '.join(SCENARIOS)}",
            file=sys.stderr,
        )
        return 2

    if not args.isolate:
        # Sem isolamento: garante o ambiente no ar uma única vez.
        reset_environment(build=args.build)

    results = []
    suite_started = time.monotonic()
    for name in selected:
        if args.isolate:
            reset_environment(build=args.build)
        started = time.monotonic()
        code = run_scenario(name)
        elapsed = time.monotonic() - started
        results.append((name, code, elapsed))

    if not args.keep_up:
        run_compose(["down", "-v"], check=False)

    total_elapsed = time.monotonic() - suite_started
    passed = sum(1 for _, code, _ in results if code == 0)
    failed = len(results) - passed

    print("\n" + "=" * 80)
    print("RESUMO DA SUÍTE DE CENÁRIOS")
    print("=" * 80)
    for name, code, elapsed in results:
        status = "PASS" if code == 0 else f"FAIL (exit {code})"
        print(f"  {status:<16} {name:<32} {elapsed:6.1f}s")
    print("-" * 80)
    print(
        f"  {passed} passaram, {failed} falharam, "
        f"{len(results)} no total em {total_elapsed:.1f}s"
    )
    print("=" * 80)

    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
