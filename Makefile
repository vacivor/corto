.PHONY: build-frontend build-backend build run

build-frontend:
	./build-frontend.sh

build-backend:
	cargo build

build: build-frontend build-backend

run:
	cargo run
