# wasm-bindgen-file-reader

This crate implements a wrapper around a `web_sys::File` that implements `Read` and `Seek`. This is useful when you have a Rust crate that expects a generic reader and want to use it in WebAssembly without loading the entire file into memory and using a `std::io::Cursor`.

Note: this only works in a web worker context because it uses the synchronous [FileReaderSync interface](https://developer.mozilla.org/en-US/docs/Web/API/FileReaderSync).

### Installation

Add to Cargo.toml:

```
wasm-bindgen-file-reader = "1"
```

### Usage

See the [demo](https://badel2.github.io/wasm-bindgen-file-reader-test/) for more information.

Rust code:

```rust
use wasm_bindgen_file_reader::WebSysFile;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;

/// Read one byte from the file at a given offset.
#[wasm_bindgen]
pub fn read_at_offset_sync(file: web_sys::File, offset: u64) -> u8 {
    let mut wf = WebSysFile::new(file);

    // Now we can seek as if this was a real file
    wf.seek(SeekFrom::Start(offset))
        .expect("failed to seek to offset");

    // Use 1-byte buffer because we only want to read one byte
    let mut buf = [0];

    // The Read API works as with real files
    wf.read_exact(&mut buf).expect("failed to read bytes");

    buf[0]
}
```

Javascript code (index.html):

```js
let myWorker = new Worker("worker.js");
document.getElementById("filepicker").addEventListener(
    "change",
    function() {
        let file = this.files[0];
        myWorker.postMessage({ file: file, offset: 0 });
        myWorker.onmessage = function(e) {
            console.log("First byte of file is: ", e.data);
        };
    },
    false
);
```

Javascript code (worker.js):

```js
onmessage = async function(e) {
    let wasm_bindgen_file_reader_test = await Rust.wasm_bindgen_file_reader_test;
    let workerResult = wasm_bindgen_file_reader_test.read_at_offset_sync(
        e.data.file,
        e.data.offset,
    );
    postMessage(workerResult);
};
```
