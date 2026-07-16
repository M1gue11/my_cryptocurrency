# Argumentos repassados aos cenários. Use PAUSE=1 para pausar antes de cada
# passo (modo interativo), ou passe flags extras via ARGS.
#   make scenarios              -> roda tudo sem pausar
#   make scenarios PAUSE=1      -> roda tudo pausando a cada passo
#   make scenarios ARGS="--propagation-wait 4"
ARGS ?=
ifeq ($(PAUSE),1)
ARGS += --pause
endif

scenarios:
	python .\sim\scenario_late_rejoin.py $(ARGS)
	python .\sim\scenario_equal_fork_then_extension.py $(ARGS)
	python .\sim\scenario_deep_reorg.py $(ARGS)
	python .\sim\scenario_shorter_fork_rejected.py $(ARGS)
	python .\sim\scenario_three_way_fork.py $(ARGS)
	python .\sim\scenario_staged_healing_bridge.py $(ARGS)

# Valida que todos os cenários executam com sucesso, como testes unitários.
# Reseta o ambiente (volumes) antes de cada cenário para garantir isolamento.
#   make test               -> roda a suíte completa, resetando entre cenários
#   make test BUILD=1       -> força rebuild da imagem Docker
#   make test ARGS="--no-isolate"
#   make test ARGS="deep_reorg late_rejoin"   -> subconjunto por nome
test:
	python .\sim\run_scenarios.py $(if $(filter 1,$(BUILD)),--build,) $(ARGS)

up:
	docker compose -f .\docker-compose.sim.yml up --build -d
