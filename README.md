# split-image-rust

縦長の画像を指定したサイズで分割する。空白判定して文字や画像が切れないように分割できる。Webページのスクリーンショットなどを見やすいサイズに分割するときに有効。

```
Usage: split-image-rust.exe [OPTIONS] <INPUT_PATH>...

Arguments:
  <INPUT_PATH>...

Options:
  -o, --output <OUTPUT_DIR>            [default: output]
  -m, --margin <MARGIN>                [default: 0]
  -H, --max-height <MAX_HEIGHT>        max height of output images [default: 2000]
      --min-height <MIN_HEIGHT>        min height of output images [default: 1000]
      --blank-height <BLANK_HEIGHT>    height to decide blank spaces [default: 30]
      --blank-var-thr <BLANK_VAR_THR>  variance threshold to decide blank spaces [default: 100.0]
      --blank-left <BLANK_LEFT>        left portion to decide blank spaces (0-100) [default: 0]
      --blank-right <BLANK_RIGHT>      right portion to decide blank spaces (0-100) [default: 100]
      --file-ext <FILE_EXT>            output file types [default: same as input] [possible values: jpg, png]
  -h, --help                           Print help
```

## コマンド実行例

`split-image-rust -m 30 -H 1500 --min-height 800 --blank-height 40 --blank-right 50 --blank-var-thr 100 input.png`

高さ1500pxで分割する。空白を幅0-50%、高さ40pxのボックスで探索して高さ800pxまでの間にあればその位置で分割する。

`--blank-var-thr`は画素値の分散で空白の判定に使用する。空白箇所は概ね分散 0 になる。
