# Rust Git CLI

[中文](README.md) | [English](README.en.md) | [Deutsch](README.de.md) | 日本語

AIを使用して中国語/英語のバイリンガルコミットメッセージを生成するインテリジェントなGitコミットツール。

## 特徴

- **AI駆動** - OpenAI、Anthropic、カスタムエンドポイント（DeepSeekなど）をサポート
- **バイリンガルコミット** - Conventional Commits規約に従った中国語/英語のコミットメッセージを自動生成
- **スマートステージング** - ステージされていない変更を検出し、確認を求める
- **インタラクティブUI** - カラー出力、差分プレビュー、コミット確認
- **AI Changelog** - コミットをインタラクティブに選択し、AI changelogサマリーを生成、クリップボードにコピー
- **AI Review** - 現在の変更に対するAIレビューを生成（リスク/提案/テスト）
- **設定管理** - `config` で設定の表示・場所確認・編集・書き換え、`config doctor` でキーとエンドポイントを自己診断
- **スクリプト化** - `commit --dry-run` はメッセージを出力するだけでコミットしない、`log --export` はファイル出力、`hook install` で通常の `git commit` にもAIを適用

## インストール

```bash
# ソースからビルド
git clone https://github.com/duanyongcheng/rust-git-cli.git
cd rust-git-cli
cargo build --release

# システムにインストール
cargo install --path .
```

**要件**: Rust 1.70+, Git 2.0+

## クイックスタート

### 1. 設定の初期化

```bash
rust-git-cli init                 # グローバル設定を作成 (~/.config/rust-git-cli/config.toml)
rust-git-cli init --local         # プロジェクト設定を作成 (.rust-git-cli.toml)
```

### 2. APIキーの設定

```bash
# 推奨：環境変数（api_key_env に入れるのは変数名で、キーそのものではありません）
export OPENAI_API_KEY="your-api-key"
# または
export ANTHROPIC_API_KEY="your-api-key"

# 設定ファイルに直接書くこともできます（パーミッション600、`config` の出力では伏字になります）
rust-git-cli config set ai.api_key "your-api-key"
```

> `api_key_env` には `OPENAI_API_KEY` のような変数名を入れます。キーを書くと `config set` は拒否し、`config doctor` が指摘します。

### 3. 使用方法

```bash
rust-git-cli                      # リポジトリの状態を確認（デフォルト）
rust-git-cli commit               # AIコミットメッセージを生成
rust-git-cli commit --show-diff   # 生成前に差分をプレビュー
rust-git-cli commit --dry-run     # メッセージを表示するだけ（ステージもコミットもしない）
rust-git-cli commit --debug       # デバッグモード
rust-git-cli review               # 現在の変更をAIでレビュー
rust-git-cli review --staged      # ステージ済みの変更のみレビュー
rust-git-cli review --base main   # main に対するブランチ全体をレビュー
rust-git-cli config doctor        # 設定・キー・エンドポイントを自己診断
```

名前が長い場合はエイリアスを：

```bash
alias vgit=rust-git-cli     # ~/.zshrc や ~/.bashrc に追記
# シェル設定を触らずに済ませるなら：ln -sf "$(command -v rust-git-cli)" ~/.local/bin/vgit
```

## コマンド

| コマンド | 説明 |
|---------|------|
| `status` | リポジトリの状態を確認（デフォルト） |
| `commit` | AIコミットを生成して実行 |
| `review` | 現在の変更をAIでレビュー |
| `diff` | コード変更を表示 |
| `log` | コミット履歴を表示、AI changelog生成とファイル出力をサポート |
| `config` | 設定の表示・場所確認・編集・書き換え、`doctor` で自己診断 |
| `hook` | `prepare-commit-msg` フックを設置/解除し、`git commit` でAIを使う |
| `init` | 設定ファイルを初期化 |

### グローバルオプション

```bash
rust-git-cli [OPTIONS] [COMMAND]

Options:
  -p, --path <PATH>    チェックするパス（デフォルト：現在のディレクトリ）
  -v, --verbose        詳細出力
  -h, --help           ヘルプを表示
  -V, --version        バージョンを表示
```

### commit オプション

```bash
rust-git-cli commit [OPTIONS]

Options:
  --api-key <KEY>      APIキーを一時的に指定
  --model <MODEL>      AIモデルを指定（例：gpt-4, deepseek-v3）
  --base-url <URL>     カスタムAPIエンドポイント
  --auto               確認をスキップして直接コミット
  --dry-run            生成したメッセージを表示するだけ（ステージもコミットもしない）
  --show-diff          生成前に差分をプレビュー
  --debug              AIの生レスポンスを表示
```

### log オプション

```bash
rust-git-cli log [OPTIONS]

Options:
  -n, --count <N>      表示するコミット数（デフォルト：10）
  --grep <PATTERN>     内容でフィルタ
  --author <NAME>      作者でフィルタ
  --since <DATE>       開始日（例："2024-01-01" または "1 week ago"）
  --until <DATE>       終了日
  --full               完全なコミットメッセージを表示
  --export <FILE>      changelogをファイルに書き出す（AI要約がない場合はコミット一覧のみ）
  --api-key <KEY>      APIキーを一時的に指定（changelog生成用）
  --model <MODEL>      AIモデルを指定
  --base-url <URL>     カスタムAPIエンドポイント
  --debug              AIの生レスポンスを表示
```

### diff オプション

```bash
rust-git-cli diff [OPTIONS]

Options:
  --staged             ステージされた変更のみ表示
```

### review オプション

```bash
rust-git-cli review [OPTIONS]

Options:
  --api-key <KEY>      APIキーを一時的に指定
  --model <MODEL>      AIモデルを指定
  --base-url <URL>     カスタムAPIエンドポイント
  --staged             ステージ済みの変更のみレビュー
  --unstaged           未ステージの変更のみレビュー
  --base <REF>         指定したrefと比較し、ブランチ全体（コミット済み未マージを含む）をレビュー
  --debug              AIの生レスポンスを表示
```

### config コマンド

設定管理はリポジトリ外でも動きます。

```bash
rust-git-cli config                  # 有効な値と、どのファイル由来かを表示
rust-git-cli config path             # 実際に読まれるファイルと、隠されている候補を表示
rust-git-cli config edit             # $VISUAL / $EDITOR で開く
rust-git-cli config open             # OSの既定アプリで開く
rust-git-cli config set ai.model deepseek-chat   # 1つのキーだけ書き換え、コメントは保持
rust-git-cli config doctor           # 解析可否・キーの出所・エンドポイント・差分の予算を自己診断
rust-git-cli config doctor --offline # ネットワーク診断をスキップ
```

`config show` は `api_key` を伏字にし（`--show-secrets` で表示）、`api_key_env` が指す変数が現在のシェルに実在するかを確認します。`config set` は書き込み前に検証し、キーを `api_key_env` に入れる・provider の綴り違い・format の綴り違いは、実行時に気づくのではなくその場で拒否されます。

### hook コマンド

```bash
rust-git-cli hook install     # .git/hooks/prepare-commit-msg を書き込む
rust-git-cli hook status      # 現在の状態を表示
rust-git-cli hook uninstall   # 解除（自身が書いたものだけ）
```

設置後、メッセージなしで `git commit` すると `rust-git-cli commit --dry-run` が呼ばれて埋められます。`-m` / `-F` / merge / squash のメッセージは上書きせず、`VGIT_NO_HOOK=1` で一時的に回避でき、他人のフックには触れません。

### init オプション

```bash
rust-git-cli init [OPTIONS]

Options:
  --local              ホームではなく現在のディレクトリに設定を作成
  --force              既存の設定を強制的に上書き
```

## 設定

設定ファイルの検索順序：
1. `./.rust-git-cli.toml`（プロジェクトレベル）
2. `~/.config/rust-git-cli/config.toml`（ユーザーレベル）
3. `~/.rust-git-cli.toml`（ユーザーレベルのフォールバック）

### 設定例

```toml
[ai]
provider = "openai"                      # openai（互換エンドポイント全般）または anthropic
model = "gpt-4"                          # モデル名
api_key_env = "OPENAI_API_KEY"           # 変数名。キーそのものではありません
# api_key = "sk-..."                     # 直接設定（非推奨だが有効）
# base_url = "https://api.deepseek.com/v1"  # カスタムエンドポイント
max_tokens = 2000                        # 初期上限。切り詰められた場合は自動で倍増

[commit]
format = "conventional"                  # conventional | plain | english
include_emoji = false                    # コミットタイプに応じたemojiを主題に付与
max_diff_size = 4000                     # AIに送信する最大差分文字数（reviewは2倍）
auto_stage = false                       # すべての変更を自動ステージ
```

### APIキーの優先順位

1. コマンドライン引数 `--api-key`
2. 設定ファイル `api_key`
3. 環境変数（`api_key_env`で指定）
4. 対話式入力

## コミットメッセージの形式

生成されるコミットメッセージは[Conventional Commits](https://www.conventionalcommits.org/)規約に従います：

```
feat(auth): 添加用户认证功能
Add user authentication feature

实现了JWT令牌验证
Implement JWT token validation
添加了用户登录接口
Add user login endpoint
```

### コミットタイプ

| タイプ | 説明 |
|--------|------|
| `feat` | 新機能 |
| `fix` | バグ修正 |
| `docs` | ドキュメント |
| `style` | コードフォーマット |
| `refactor` | リファクタリング |
| `test` | テスト |
| `chore` | ビルド/ツール |
| `perf` | パフォーマンス |

### 出力フォーマット

`commit.format` が描画方法を決めます。いずれもバイリンガルの本文は保持します：

| 値 | 出力 |
|----|------|
| `conventional` | `feat(auth): 添加用户认证功能` + `Add user authentication feature` |
| `plain` | `添加用户认证功能` + `Add user authentication feature`（タイプ接頭辞なし） |
| `english` | `feat(auth): Add user authentication feature`（英語のみ） |

`commit.include_emoji = true` で主題にemojiを付けます：`feat` ✨、`fix` 🐛、`docs` 📝、`style` 💄、`refactor` ♻️、`perf` ⚡️、`test` ✅、`build` 📦、`ci` 👷、`chore` 🔧、`revert` ⏪。未知のタイプには付けません。全行に飾りが増えるだけだからです。

## ワークフロー

```bash
# 1. 状態を確認
$ rust-git-cli status

# 2. 変更を表示
$ rust-git-cli diff

# 3. コミットを生成
$ rust-git-cli commit

# ステージされていない変更が検出された場合：
# Unstaged changes detected:
# ──────────────────────────────────────────────────
#   M src/main.rs
#   ? src/new_file.rs
# ──────────────────────────────────────────────────
# Do you want to stage all changes (git add .)? (Y/n)

# AI生成後、アクションを選択：
# - Accept and commit: 承認してコミット
# - Edit message: 編集してからコミット
# - Regenerate: 再生成
# - Cancel: キャンセル

# 4. Changelogを生成
$ rust-git-cli log -n 20

# コミットをインタラクティブに選択（Spaceで選択、Enterで確認）
# 選択後、AI changelogサマリーを生成
# クリップボードへのコピーをサポート
```

## トラブルシューティング

| 問題 | 解決策 |
|------|--------|
| API接続失敗 | ネットワークを確認、APIキーを検証、`--debug`で詳細を確認 |
| JSON解析エラー | `--debug`で生レスポンスを確認、別のモデルを試す |
| 設定が機能しない | まず `config doctor`。解析失敗時は既定値に黙って戻らず警告を出します |
| キーを設定したのに「未設定」と言われる | キーを `api_key_env` に入れていませんか。あれは変数名の欄です。`config set ai.api_key <key>` を使ってください |
| コミット失敗 | Gitユーザーが設定されていることを確認（`git config user.name/email`） |

```bash
# デバッグモード
rust-git-cli commit --debug

# ヘルプを表示
rust-git-cli --help
rust-git-cli commit --help
```

## 開発

```bash
cargo build                       # ビルド
cargo test                        # テスト
cargo fmt                         # フォーマット
cargo clippy -- -D warnings       # リント
```

### プロジェクト構造

```
src/
├── main.rs          # エントリーポイントとコマンドディスパッチ
├── cli.rs           # CLI定義（clap）
├── config.rs        # 設定管理
├── git.rs           # Git操作（git2）
├── ui.rs            # インタラクティブUI（dialoguer）
└── ai/
    ├── mod.rs       # AIクライアント抽象化
    ├── openai.rs    # OpenAI実装
    └── anthropic.rs # Anthropic実装
```

## ライセンス

MIT License

## クレジット

- [git2-rs](https://github.com/rust-lang/git2-rs) - Git操作
- [clap](https://github.com/clap-rs/clap) - コマンドライン解析
- [dialoguer](https://github.com/console-rs/dialoguer) - インタラクティブUI
- [colored](https://github.com/colored-rs/colored) - カラー出力
