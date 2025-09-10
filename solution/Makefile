.PHONY: all clean

all:
	cargo build --release
	cp target/release/bot ./bot

clean:
	cargo clean
	rm -f bot

run:
	cargo run

test-bender:
	../docker_image/m1_game_engine -f ../docker_image/maps/map01 -p1 ./bot -p2 ../docker_image/m1_robots/bender

test-walle:
	../docker_image/m1_game_engine -f ../docker_image/maps/map01 -p1 ./bot -p2 ../docker_image/m1_robots/wall_e
