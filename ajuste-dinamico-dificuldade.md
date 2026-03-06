# Plano: Ajuste Dinâmico de Dificuldade de Mineração

## Context

Atualmente a dificuldade é um valor fixo (`8` bits) em `CONSENSUS_RULES.difficulty` (global estático). Não existe ajuste automático, ou seja, a dificuldade nunca muda independente de quantos mineradores estão ativos ou de quão rápido os blocos estão sendo minerados.

O objetivo é:
1. Salvar a dificuldade vigente em cada `BlockHeader` (para que validação histórica e consenso em rede funcionem corretamente)
2. Recalcular a dificuldade a cada **5 blocos** com base no tempo real levado, apontando para **10 segundos por bloco** de tempo médio
3. Algoritmo proporcional: `new_bits = max(1, round(d - log2(actual / expected)))` onde `d` é a dificuldade atual em bits, `actual` o tempo real do período e `expected = 5 * 10 = 50 segundos`

> **Nota**: O arquivo `bc.json` existente precisará ser apagado ao testar, pois blocos antigos não têm o campo `difficulty` no header (o campo usará valor padrão via serde, mas a validação da sequência de dificuldades falhará para cadeias antigas).

---

## Parâmetros (configurados em `CONSENSUS_RULES`)

| Parâmetro | Valor | Descrição |
|-----------|-------|-----------|
| `difficulty` | 8 | Dificuldade inicial (genesis) |
| `difficulty_adjustment_interval` | 5 | A cada quantos blocos ajustar |
| `target_block_time_secs` | 10 | Tempo alvo em segundos por bloco |

---

## Arquivos Críticos

- [`project/src/globals.rs`](project/src/globals.rs) — ConsensusRules
- [`project/src/model/block.rs`](project/src/model/block.rs) — BlockHeader, Block::new(), validate(), header_bytes()
- [`project/src/model/miner.rs`](project/src/model/miner.rs) — build_block(), mine()
- [`project/src/model/blockchain.rs`](project/src/model/blockchain.rs) — add_block(), novo método calculate_next_difficulty()
- [`project/src/model/node.rs`](project/src/model/node.rs) — mine(), validate_blockchain()

---

## Passos de Implementação

### 1. `src/globals.rs` — Adicionar parâmetros de ajuste em `ConsensusRules`

```rust
pub struct ConsensusRules {
    pub difficulty: usize,
    pub max_block_size_kb: f32,
    pub block_reward: i64,
    pub difficulty_adjustment_interval: usize,   // NOVO
    pub target_block_time_secs: u64,             // NOVO
}

pub static CONSENSUS_RULES: Lazy<ConsensusRules> = Lazy::new(|| ConsensusRules {
    difficulty: 8,
    max_block_size_kb: 1.0,
    block_reward: 1 * COIN,
    difficulty_adjustment_interval: 5,           // NOVO
    target_block_time_secs: 10,                  // NOVO
});
```

---

### 2. `src/model/block.rs` — Adicionar campo `difficulty` ao `BlockHeader`

**a) Struct `BlockHeader`**: adicionar `difficulty: usize` com default serde para retrocompatibilidade:

```rust
fn default_difficulty() -> usize {
    CONSENSUS_RULES.difficulty
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockHeader {
    pub prev_block_hash: BlockID,
    pub merkle_root: BlockID,
    pub nonce: u32,
    pub timestamp: NaiveDateTime,
    #[serde(default = "default_difficulty")]
    pub difficulty: usize,                       // NOVO
}
```

**b) `Block::new()`**: receber `difficulty` como parâmetro:

```rust
pub fn new(prev_block_hash: BlockID, difficulty: usize) -> Self {
    // ...
    BlockHeader { prev_block_hash, merkle_root: [0;32], nonce: 0, timestamp, difficulty }
}
```

**c) `header_bytes()`**: incluir `difficulty` na serialização do header (afeta o hash, o que é correto — a dificuldade faz parte do comprometimento do bloco):

```rust
out.extend_from_slice(&(self.header.difficulty as u64).to_be_bytes());
```

**d) `validate()`**: usar `self.header.difficulty` em vez de `CONSENSUS_RULES.difficulty`:

```rust
if !hash_starts_with_zero_bits(&self.header_hash(), self.header.difficulty) {
    return Err("Invalid proof of work".to_string());
}
```

---

### 3. `src/model/miner.rs` — Passar `difficulty` para `build_block()`

**a) `build_block()`**: adicionar parâmetro `difficulty` e passar para `Block::new()`:

```rust
fn build_block(&mut self, mempool: &Vec<MempoolTx>, previous_hash: [u8;32], difficulty: usize) -> Block {
    // ...
    let mut new_block = Block::new(previous_hash, difficulty);   // alterado
    // ...
}
```

**b) `mine()`**: passar `difficulty` para `build_block()`:

```rust
let mut block_to_mine = self.build_block(mempool, previous_hash, difficulty);
```

A assinatura de `mine()` já recebe `difficulty` — sem alteração necessária na assinatura.

---

### 4. `src/model/blockchain.rs` — Calcular próxima dificuldade + validar no `add_block()`

**a) Novo método `calculate_next_difficulty(&self) -> usize`**:

Calcula a dificuldade esperada para o próximo bloco (height = `self.chain.len()`):

```rust
pub fn calculate_next_difficulty(&self) -> usize {
    let height = self.chain.len();
    let interval = CONSENSUS_RULES.difficulty_adjustment_interval;

    // Antes do primeiro período de ajuste ou não é momento de ajuste
    if height == 0 || height % interval != 0 {
        return self.chain.last()
            .map(|b| b.header.difficulty)
            .unwrap_or(CONSENSUS_RULES.difficulty);
    }

    // É momento de ajuste: analisar os últimos `interval` blocos
    let period_start = &self.chain[height - interval];
    let period_end   = &self.chain[height - 1];

    let actual_secs = period_end.header.timestamp
        .signed_duration_since(period_start.header.timestamp)
        .num_seconds()
        .max(1) as f64;  // evitar divisão por zero

    let expected_secs = (interval as u64 * CONSENSUS_RULES.target_block_time_secs) as f64;

    let current_d = period_end.header.difficulty as f64;
    let new_d = (current_d - (actual_secs / expected_secs).log2()).round() as isize;

    // Limitar: mínimo 1 bit, máximo current_d + 4 (cap de segurança)
    new_d.max(1).min(current_d as isize + 4) as usize
}
```

**b) `add_block()`**: após validação do bloco, verificar se a dificuldade declarada é a correta:

```rust
let expected_difficulty = self.calculate_next_difficulty();
if block.header.difficulty != expected_difficulty {
    return Err(format!(
        "Invalid difficulty: expected {}, got {}",
        expected_difficulty, block.header.difficulty
    ));
}
```

---

### 5. `src/model/node.rs` — Usar dificuldade dinâmica ao minerar e validar

**a) `mine()`**: calcular dificuldade antes de minerar:

```rust
pub fn mine(&mut self) -> Result<&Block, String> {
    let previous_hash = self.blockchain.get_last_block_hash();
    let difficulty = self.blockchain.calculate_next_difficulty();  // NOVO
    self.difficulty = difficulty;  // atualizar campo (opcional, para logs/inspeção)

    let mined_block = self.miner.mine(&self.mempool, previous_hash, difficulty)?;
    // ...
}
```

**b) `validate_blockchain()`**: verificar a sequência de dificuldades ao recarregar a cadeia do disco. Construir uma `Blockchain` temporária para recalcular o expected a cada passo:

```rust
fn validate_blockchain(bc: &Blockchain) -> Result<bool, String> {
    let chain_ref = &bc.chain;
    let mut partial = Blockchain::new();  // cadeia crescente para cálculo

    for (i, block) in chain_ref.iter().enumerate() {
        if i == 0 {
            if block.header.prev_block_hash != [0; 32] {
                return Err("Genesis block has invalid previous hash".to_string());
            }
            // Para o bloco genesis não verificamos a sequência (é o ponto de partida)
            partial.chain.push(block.clone());
            continue;
        }

        let expected_difficulty = partial.calculate_next_difficulty();
        if block.header.difficulty != expected_difficulty {
            return Err(format!(
                "Block {} has invalid difficulty: expected {}, got {}",
                i, expected_difficulty, block.header.difficulty
            ));
        }

        if let Err(e) = block.validate() {
            return Err(e);
        }

        let prev_block = &chain_ref[i - 1];
        if block.header.prev_block_hash != prev_block.header_hash() {
            return Err(format!("Block {} has invalid previous block hash", i));
        }

        partial.chain.push(block.clone());
    }
    Ok(true)
}
```

---

## Algoritmo de Ajuste — Resumo Visual

```
Período = 5 blocos, alvo = 10s/bloco → esperado = 50s no total

Se minerou em 25s (muito rápido):
  ratio = 25/50 = 0.5  →  log2(0.5) = -1  →  new_d = 8 - (-1) = 9 bits

Se minerou em 100s (muito lento):
  ratio = 100/50 = 2.0  →  log2(2.0) = 1.0  →  new_d = 8 - 1 = 7 bits

Se minerou em 50s (no alvo):
  ratio = 1.0  →  log2(1.0) = 0  →  new_d = 8 bits (sem mudança)
```

---

## Verificação (como testar)

1. Apagar `saved_files/bc.json` e `saved_files/mempool.json`
2. Subir o nó e minerar blocos continuamente
3. Após o 5º bloco, o log deve mostrar a dificuldade sendo recalculada
4. Inspecionar via API HTTP (`GET /blocks`) e verificar que cada bloco a partir do 5º tem `difficulty` diferente se o tempo foi desviante
5. Tentar submeter via rede um bloco com `difficulty` incorreta → deve ser rejeitado pelo `add_block()`
6. Reiniciar o nó (recarregar `bc.json`) → `validate_blockchain()` deve passar sem erros
