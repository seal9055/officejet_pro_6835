all:
	cargo run --release
	gcc lzss.c -o lzss
	./lzss

clean:
	cargo clean
	rm lzss firmware data1 data2 data3
