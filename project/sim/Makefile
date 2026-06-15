.PHONY: all reset run up down clean-data

ROOT_DIR := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/..)
COMPOSE := docker compose -f "$(ROOT_DIR)/docker-compose.sim.yml"
DATA_DIR := $(ROOT_DIR)/.sim-data

ROUNDS ?= 1000
SEED ?= 11

all: reset run

reset: clean-data up

run:
	cd "$(ROOT_DIR)" && python sim/orchestrator.py --rounds $(ROUNDS) --seed $(SEED)

up:
	$(COMPOSE) up -d

down:
	$(COMPOSE) down --remove-orphans

clean-data: down
	powershell -NoProfile -ExecutionPolicy Bypass -Command "Remove-Item -LiteralPath '$(DATA_DIR)' -Recurse -Force -ErrorAction SilentlyContinue"
