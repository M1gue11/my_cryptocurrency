#!/usr/bin/env python3
"""
Gera figuras do experimento distribuido do Caramuru a partir de um results.json.

Uso:
    python3 sim/plot_results.py --input sim/results.json --outdir sim/figs

Figuras geradas (PNG, prontas para o relatorio):
  1. altura_por_no.png        -> altura da cadeia por no ao longo das rodadas (convergencia)
  2. blocos_por_no.png        -> blocos minerados aceitos por no (autoria)
  3. tempo_entre_blocos.png   -> segundos por bloco ao longo da cadeia
  4. transacoes.png           -> submetidas vs aceitas vs falhas (por no)
  5. saldos_finais.png        -> saldo final da carteira mineradora por no

Sem dependencia alem de matplotlib.
"""
import argparse
import json
import os


def load(path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def fig_altura(d, outdir):
    import matplotlib.pyplot as plt

    hist = d["history"]
    nodes = list(d["summary"]["final_snapshot"]["nodes"].keys())
    rounds = [h["round"] for h in hist]
    styles = ["-", "--", "-.", ":"]
    markers = ["o", "s", "^", "D"]
    # offset vertical sutil por no para revelar a sobreposicao (as linhas
    # coincidem porque a rede converge; o jitter so torna isso visivel).
    offsets = [0.06, 0.02, -0.02, -0.06]
    fig, ax = plt.subplots(figsize=(8, 4.5))
    for i, n in enumerate(nodes):
        ys = [h["snapshot"]["nodes"].get(n, {}).get("height") for h in hist]
        ys_off = [(y + offsets[i] if y is not None else None) for y in ys]
        ax.plot(rounds, ys_off, marker=markers[i], markersize=4,
                linestyle=styles[i], linewidth=1.4, alpha=0.85, label=n)
    # marca rodadas divergentes
    div = d["summary"].get("convergence", {}).get("divergent_rounds", [])
    for j, r in enumerate(div):
        ax.axvline(r, color="red", linestyle="--", alpha=0.35, linewidth=1,
                   label="rodada divergente" if j == 0 else None)
    ax.set_xlabel("Rodada")
    ax.set_ylabel("Altura da cadeia")
    ax.set_title("Convergência de altura por nó\n(offset vertical leve, apenas visual, para revelar as 4 séries sobrepostas)")
    ax.legend(loc="lower right", fontsize=8)
    ax.grid(True, alpha=0.3)
    _save(fig, outdir, "altura_por_no.png")


def fig_blocos(d, outdir):
    import matplotlib.pyplot as plt

    bm = d["summary"]["blocks_mined_by_node"]
    nodes = list(bm.keys())
    vals = [bm[n] for n in nodes]
    fig, ax = plt.subplots(figsize=(6, 4))
    bars = ax.bar(nodes, vals, color="#4C72B0")
    ax.bar_label(bars)
    ax.set_ylabel("Blocos minerados aceitos")
    ax.set_title("Autoria de blocos na cadeia final por no")
    ax.grid(True, axis="y", alpha=0.3)
    _save(fig, outdir, "blocos_por_no.png")


def fig_block_time(d, outdir):
    import matplotlib.pyplot as plt

    bt = d["summary"].get("block_time", {})
    samples = bt.get("samples", [])
    if not samples:
        return
    xs = [s["to_height"] for s in samples]
    ys = [s["seconds_per_block"] for s in samples]
    avg = bt.get("average_seconds_between_blocks")
    fig, ax = plt.subplots(figsize=(8, 4))
    ax.plot(xs, ys, marker="o", color="#55A868", linewidth=1.4)
    if avg is not None:
        ax.axhline(avg, color="gray", linestyle="--", label=f"media = {avg:.3f}s")
        ax.legend(fontsize=8)
    ax.set_xlabel("Altura da cadeia")
    ax.set_ylabel("Segundos por bloco")
    ax.set_title("Tempo entre blocos aceitos")
    ax.grid(True, alpha=0.3)
    _save(fig, outdir, "tempo_entre_blocos.png")


def fig_transacoes(d, outdir):
    import matplotlib.pyplot as plt

    tx = d["summary"]["events"]["tx_by_node"]
    nodes = list(tx.keys())
    sub = [tx[n]["submitted"] for n in nodes]
    acc = [tx[n]["accepted"] for n in nodes]
    fail = [tx[n]["failed"] for n in nodes]
    x = range(len(nodes))
    w = 0.27
    fig, ax = plt.subplots(figsize=(7, 4))
    ax.bar([i - w for i in x], sub, w, label="submetidas", color="#4C72B0")
    ax.bar(list(x), acc, w, label="aceitas", color="#55A868")
    ax.bar([i + w for i in x], fail, w, label="falhas", color="#C44E52")
    ax.set_xticks(list(x))
    ax.set_xticklabels(nodes)
    ax.set_ylabel("Transacoes")
    ax.set_title("Transacoes por no: submetidas vs aceitas vs falhas")
    ax.legend(fontsize=8)
    ax.grid(True, axis="y", alpha=0.3)
    _save(fig, outdir, "transacoes.png")


def fig_saldos(d, outdir):
    import matplotlib.pyplot as plt

    bf = d["summary"]["balances_final"]
    nodes = list(bf.keys())
    vals = [bf[n] / 1_000_000 for n in nodes]  # COIN = 1_000_000
    fig, ax = plt.subplots(figsize=(6, 4))
    bars = ax.bar(nodes, vals, color="#8172B3")
    ax.bar_label(bars, fmt="%.2f")
    ax.set_ylabel("Saldo final (CRU)")
    ax.set_title("Saldo final da carteira mineradora por no")
    ax.grid(True, axis="y", alpha=0.3)
    _save(fig, outdir, "saldos_finais.png")


def _save(fig, outdir, name):
    fig.tight_layout()
    path = os.path.join(outdir, name)
    fig.savefig(path, dpi=150)
    print(f"  figura: {path}")


def main():
    ap = argparse.ArgumentParser(description="Gera figuras do experimento distribuido")
    ap.add_argument("--input", default="sim/results.json")
    ap.add_argument("--outdir", default="sim/figs")
    args = ap.parse_args()
    os.makedirs(args.outdir, exist_ok=True)
    d = load(args.input)
    print(f"Gerando figuras a partir de {args.input} (seed={d['summary'].get('seed')}, "
          f"rounds={d['summary'].get('rounds')}):")
    fig_altura(d, args.outdir)
    fig_blocos(d, args.outdir)
    fig_block_time(d, args.outdir)
    fig_transacoes(d, args.outdir)
    fig_saldos(d, args.outdir)
    print("Concluido.")


if __name__ == "__main__":
    main()
