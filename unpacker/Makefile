all:
	cargo run --release
	gcc lzss.c -m32 -o lzss
	gcc lzss.c -o lzss
	./lzss

clean:
	cargo clean
	rm lzss firmware data1 data2 data3 data1b data2b data2c data1c data2c data3c
