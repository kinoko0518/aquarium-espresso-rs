# Aquarium Espresso (Rust)

![Aquarium Espresso](screenshot.jpg)

ESP32-S3向けに制作された癒やしの熱帯魚水槽「Aquarium Espresso」を、Rust + Macroquad へ移植したデスクトップ向けアクアリウムです。
Linux (Hyprland / hyprwinwrap、X11、Wayland) などの環境で、デスクトップ壁紙やスクリーンセーバー、ウィンドウとして鑑賞できます。

グッピー、ネオンテトラ、ブラックテトラ、コリドラスのほか、稀にヤマトヌマエビも出現します。
（時にはエビフライが泳ぎだすことも……？）

基本的には眺めるだけの癒やしプログラムですが、水面をクリックして波紋を起こしたり、背景を切り替えたりできます。

---

## 主な機能

- **波紋・コースティクス光響レンダリング**: 水面揺らぎとコースティクス（集光模様）をリアルタイムに計算。
- **魚たちの自律行動**: 群れや単独で優雅に泳ぎ回るフィッシュAI。
- **5種類の背景水槽**: 起動時の指定やホットキーによるリアルタイム切り替え。
- **オリジナルキャラクターの遊泳**: カレントディレクトリに `fish.png` を配置すると、オリジナルキャラが水槽内を泳ぎます。
- **インタラクティブ操作**: マウスクリックで水面をタップして魚たちの反応を楽しめます。
- **低負荷設計**: hyprwinwrap 等での常駐壁紙利用を考慮し、デフォルト 24 FPS 制限およびゼロアロケーション描画パイプラインを実装。

---

## 起動・操作方法

### コマンドライン引数

```bash
# デフォルト（背景ランダム、24 FPS）
cargo run --release

# 背景番号（0〜4）を指定して起動
cargo run --release -- --bg 2

# フレームレートを指定して起動（例: 30 FPS や 60 FPS）
cargo run --release -- --fps 30
```

### 操作キー

| キー / 操作 | 動作 |
|---|---|
| `Space` / `B` | 背景画像を順番に切り替え（全5種） |
| `R` | シミュレーションのリセット（魚や泡を初期状態に戻す） |
| `E` | エビフライモード（グッピーがエビフライに変身） |
| `左クリック` | 水面をタップ（波紋が発生し、近くの魚が驚いて逃げます） |
| `Q` / `Esc` | 終了 |

### オリジナルキャラを泳がせる

透過PNG画像（推奨: `126px x 128px` 程度）を `fish.png` という名前で実行ディレクトリに配置すると、グッピーの一部がオリジナルキャラクターに置き換わって水槽内を泳ぎます。

---

## ビルド方法

### Nix を使用する場合 (推奨)

Nix Flakes を使用して簡単にビルド・実行できます。

```bash
# 直接実行
nix run

# 開発環境に入る
nix develop
cargo run --release
```

### 通常の Cargo でビルドする場合

以下のライブラリ（グラフィックス・ウィンドウ関連）が必要です。

- **Debian / Ubuntu**:
  `sudo apt install pkg-config libx11-dev libxi-dev libgl1-mesa-dev libxcursor-dev libxrandr-dev libxkbcommon-dev`
- **Arch Linux**:
  `sudo pacman -S libx11 libxi mesa libxcursor libxrandr libxkbcommon`

```bash
cargo build --release
```

バイナリは `target/release/aquarium-espresso-rs` に生成されます。

---

## hyprwinwrap（Hyprlandの背景壁紙化）での利用例

Hyprland 環境において、`hyprwinwrap` プラグインを使ってデスクトップの動的壁紙として常駐させることができます。

`hyprland.conf`:
```conf
plugin {
    hyprwinwrap {
        # 対象ウィンドウクラス
        class = aquarium-espresso-rs
    }
}

# 起動例 (exec-once)
exec-once = aquarium-espresso-rs
```

---

## ライセンス・クレジット

- **オリジナル版作者**: もちもちまん / Uh ([@calorie0](https://x.com/calorie0))
- **ライセンス**: [MIT License](LICENSE)
- 同梱の背景画像等は AI（Grok, Gemini）により生成されたものです。
