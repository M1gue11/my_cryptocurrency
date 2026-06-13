# Ambiente de simulação distribuída do Caramuru — Nível 3

Experimento distribuído **reproduzível** com 4 nós em contêineres, orquestrados
por um script externo com **seed fixa**. O objetivo é observar comportamento
funcional da rede — sincronização, propagação de blocos/transações e
convergência da cadeia — e não medir throughput/carga formal.

## Componentes

- `../docker-compose.sim.yml` — sobe `node1..node4` em modo `daemon http`, cada
  um com `BIND_ADDR=0.0.0.0` para aceitar JSON-RPC externo. Topologia P2P em
  malha parcial (anel + ponte).
- `orchestrator.py` — cliente JSON-RPC (stdlib pura) que executa ações
  pseudoaleatórias controladas por seed e coleta métricas por rodada.

## Pré-requisitos

- Docker + Docker Compose v2
- Python 3 (somente biblioteca padrão)

## Como rodar

```bash
# a partir de project/
docker compose -f docker-compose.sim.yml up --build -d

# aguarda os nós, cria carteiras de minerador e roda o experimento
python3 sim/orchestrator.py --rounds 30 --seed 42

# encerrar e limpar volumes
docker compose -f docker-compose.sim.yml down -v
```

## Portas publicadas

| Nó    | HTTP/RPC (host) | P2P (host) |
|-------|-----------------|------------|
| node1 | 7101            | 6101       |
| node2 | 7102            | 6102       |
| node3 | 7103            | 6103       |
| node4 | 7104            | 6104       |

## Ações da simulação

Pesos fixos, sorteados por `random.Random(seed)`:

- `mine` (0.55) — minera 1 bloco num nó aleatório
- `send` (0.30) — envia transação do minerador de um nó para o endereço de outro
- `query` (0.15) — consulta o mempool (ação observacional)

A cada rodada o orquestrador tira um *snapshot* de `block_height`,
`top_block_hash` e `peers_connected` de cada nó, e ao final verifica
**convergência** (mesma altura e mesmo top-hash em todos os nós).

## Saída

`sim/results.json` com:

- `summary`: seed, rounds, convergência final, snapshot final
- `history`: evento e snapshot de cada rodada

Exit code: `0` se convergiu, `2` se não convergiu ao final.

## Reprodutibilidade

A mesma `--seed` gera a mesma sequência de ações. Para variar cenários sem
perder reprodutibilidade, basta trocar a seed e registrar o valor usado.
