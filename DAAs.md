## Resumo dos Algoritmos de Ajuste de Dificuldade (DAAs)

O ponto de partida clássico é o **Bitcoin DAA (Difficulty Adjustment Algorithm)** original descrito por Nakamoto (2008). Ele recalcula a dificuldade a cada 2016 blocos, comparando o tempo real que levou para minerar esses blocos com o tempo esperado (2 semanas). É simples e robusto, mas extremamente lento para reagir — um atacante pode explorar essa inércia, e moedas menores (com hashrate volátil) sofrem com oscilações bruscas entre blocos muito rápidos e muito lentos.

Para o seu caso — moeda nova, hashrate potencialmente instável, e contexto acadêmico — algoritmos mais reativos são a escolha certa. Os principais candidatos são:

**DigiShield v3** — criado pelo projeto DigiByte em 2014 e depois adotado pelo Dogecoin. Ele ajusta a dificuldade a cada bloco usando uma média dos últimos blocos, mas aplica um "damping factor" para evitar oscilações excessivas. A referência primária aqui é o DigiByte Security paper e as discussões técnicas de Jared Tate (fundador do DigiByte).

**Dark Gravity Wave (DGW)** — criado por Evan Duffield para o Dash (2014). Usa uma média móvel ponderada exponencialmente dos últimos ~24 blocos. É mais reativo que o Bitcoin DAA mas pode apresentar instabilidade em cenários de hashrate muito volátil.

**LWMA (Linear Weighted Moving Average)** — proposto por zawy12 (pseudônimo, pesquisador independente amplamente citado na comunidade de criptomoedas). Este é provavelmente o mais adequado para seu caso. Ele calcula a dificuldade com base nos últimos _N_ blocos, atribuindo peso linear crescente aos blocos mais recentes: blocos recentes influenciam mais o cálculo do que blocos antigos. Foi adotado por dezenas de altcoins (Masari, LOKI/Oxen, entre outras). A referência central é o repositório zawy12/difficulty-algorithms no GitHub, que contém análise matemática detalhada e comparações com outros algoritmos.

**ASERT (Absolutely Scheduled Exponential Rising Targets)** — adotado pelo Bitcoin Cash em 2020 (aserti3-412d). Usa uma fórmula exponencial que calcula a dificuldade com base no desvio entre o timestamp real e o timestamp "ideal" de cada bloco. É matematicamente elegante e resistente a manipulação de timestamps. A referência é o paper de Mark Lundeberg (2020), "ASERT: Absolutely Scheduled Exponential Rising Targets", e o CHIP-2020-07-DAA do Bitcoin Cash.

## Recomendação: LWMA

Para o seu TCC, eu recomendo o **LWMA** pelos seguintes motivos: é altamente reativo (ajusta a cada bloco), a matemática é intuitiva e fácil de explicar em um trabalho acadêmico, tem ampla adoção validando sua eficácia, e o zawy12 fornece análise comparativa extensa contra outros algoritmos.

A ideia central é simples. Dado uma janela de _N_ blocos, você calcula um "solvetime" ponderado:

```
t = Σ(i=1 até N) de: solvetime[i] * i
```

Cada solvetime é multiplicado pelo seu índice (peso linear), então o bloco mais recente tem peso _N_, o anterior _N-1_, e assim por diante. A nova dificuldade é então:

```
next_difficulty = avg_difficulty * target_solvetime * N * (N+1) / (2 * t)
```

Onde `N*(N+1)/2` é a soma dos pesos. Isso faz com que a dificuldade se ajuste proporcionalmente ao desvio do tempo de mineração observado em relação ao tempo alvo, com ênfase nos blocos mais recentes.

Também é importante aplicar limites (clamps) para evitar saltos absurdos — tipicamente limitando o ajuste a algo entre 0.5x e 2x da dificuldade anterior por bloco.

## Sobre o Tempo Médio de Bloco (Target Solvetime)

Essa é uma decisão de design fundamental que envolve trade-offs reais:

**Blocos mais rápidos** (ex: 15s–60s) dão melhor experiência ao usuário com confirmações mais rápidas, mas aumentam a **taxa de blocos órfãos** (stale blocks). Quando dois mineradores encontram um bloco quase ao mesmo tempo, a rede precisa de tempo para propagar esse bloco antes que outro seja encontrado. Se o block time for menor que o tempo de propagação, você terá forks frequentes, o que desperdiça trabalho computacional e reduz a segurança efetiva da rede. O paper de Decker e Wattenhofer (2013), "Information Propagation in the Bitcoin Network", mediu o tempo de propagação na rede Bitcoin em torno de 6–12 segundos para alcançar a maior parte dos nós. Para uma rede menor e controlada como a do seu TCC, esse tempo será muito menor, mas o princípio se mantém.

**Blocos mais lentos** (ex: 5–10 min) são mais seguros e têm taxa de órfãos negligível, mas a experiência do usuário sofre com esperas longas.

Uma boa referência aqui é a análise do Ethereum, que escolheu ~13-15 segundos justamente explorando esse limite inferior, mas precisou implementar o protocolo GHOST (Sompolinsky & Zohar, 2015, "Secure High-Rate Transaction Processing in Bitcoin") para lidar com a alta taxa de tios/órfãos resultante.

Para seu TCC, um target de **60 a 120 segundos** é um bom ponto de equilíbrio — rápido o suficiente para demonstrações práticas, lento o suficiente para não precisar lidar com alta taxa de órfãos, e fácil de argumentar academicamente. Você pode inclusive dedicar uma seção do trabalho justificando essa escolha com base nos trade-offs mencionados.

## Referências para o TCC

Aqui estão as referências que mencionei de forma organizada:

- **Nakamoto, S. (2008).** "Bitcoin: A Peer-to-Peer Electronic Cash System" — base do DAA original
- **Decker, C. & Wattenhofer, R. (2013).** "Information Propagation in the Bitcoin Network" — propagação e impacto no block time
- **Sompolinsky, Y. & Zohar, A. (2015).** "Secure High-Rate Transaction Processing in Bitcoin" (GHOST protocol) — trade-offs de block times curtos
- **zawy12. difficulty-algorithms (GitHub)** — análise comparativa de DAAs incluindo LWMA
- **Lundeberg, M. (2020).** "ASERT" e CHIP-2020-07 — alternativa exponencial elegante
- **Meshkov, D., Chepurnoy, A., & Jansen, M. (2017).** "Short Paper: Revisiting Difficulty Control for Blockchain Systems" — análise formal de algoritmos de ajuste de dificuldade (este é especialmente bom para TCC por ser um paper acadêmico formal)
