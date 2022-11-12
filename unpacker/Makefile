all:
	cargo run --release
	gcc lzss.c -O2 -o lzss
	./lzss

clean:
	cargo clean
	rm lzss firmware data* crom*
