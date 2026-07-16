<div align="center">
<sub>🇬🇧 <a href="./README.md">English</a> &nbsp;|&nbsp; 🇨🇳 <a href="./README.zh-CN.md">中文</a> &nbsp;|&nbsp; 🇪🇸 <a href="./README.es.md">Español</a> &nbsp;|&nbsp; 🇯🇵 <a href="./README.ja.md">日本語</a> &nbsp;|&nbsp; 🇧🇷 <a href="./README.pt.md">Português</a></sub>
</div>

<br/>

<div align="center">

<img src="logo.png" alt="Perch" width="120">

# Perch

**Windows 向けプロセス & サービスマネージャー。軽量・シングルバイナリ・Web UI 内蔵。**

Rust で記述。ランタイム依存なし。バイナリ1つ、ダブルクリックするだけ。

</div>

---

## スクリーンショット

> スクリーンショットファイルを以下のパスに配置してください（[提供するスクリーンショットファイル](#提供するスクリーンショットファイル) を参照）。

### インタラクティブメニュー（`perch.exe` をダブルクリック）

![menu](docs/screenshots/menu.png)

### ログイン

![login](docs/screenshots/login.png)

### プロセスダッシュボード

![processes](docs/screenshots/processes.png)

### プロセス詳細 — ライブログストリーミング

![process-detail](docs/screenshots/process-detail.png)

### スケジュールタスク

![tasks](docs/screenshots/tasks.png)

---

## なぜ Perch？

Windows でバックグラウンドプロセスや長期間稼働するサービスを管理するのは、思ったより面倒です：

- **Task Scheduler** はデフォルトで長期間稼働するプロセスを強制終了します（セッションタイムアウト、プロセスツリーの終了、ログインセッションへの依存）— もともと daemon 向けに作られていません。
- **NSSM / WinSw / Shawl** はサービスごとに1つの wrapper が必要。ダッシュボードも中央ビューもなく、N 回登録しなければなりません。
- **PM2** は Node.js ランタイムが必要で、daemon がアイドル時でも 40–80 MB を消費し、Windows では二級市民です（起動が不安定、レジストリのハックが必要）。
- **Supervisor** は Windows では動作しません。

Perch がこの隙間を埋めます：**一度登録すれば、すべてを管理。** 単一のネイティブサービスがすべてのプロセスを監視し、Web UI を提供し、目立たず動作します。

## 機能

- **シングルバイナリ** — ランタイム不要、依存関係なし。Web UI は実行ファイルに直接組み込まれています。
- **ネイティブ Windows サービス** — 一度登録すれば、起動時に開始、ログアウト後も継続、Session 0 で実行。
- **低メモリ使用量** — Rust でコンパイル、単一の supervisor プロセス。常駐メモリ 5–15 MB を目標。
- **Web UI 内蔵** — ブラウザからステータス確認、プロセスの開始/停止/再起動、ログの tail が可能。コンソールだけでなくリモート管理に対応。
- **自動再起動** — クラッシュしたプロセスは自動的に再起動され、バックオフは設定可能。
- **スケジュールタスク** — cron 形式の周期ジョブのスケジューリング。
- **日次ログローテーション** — アプリケーションログは実行ファイルと同じ場所の `logs/perch.YYYY-MM-DD.log` に書き込まれ、パイプ区切り形式。
- **インタラクティブメニュー** — exe をダブルクリックして、サービスのインストール/アンインストール/開始/停止、Web UI の起動が可能。UAC 昇格は必要に応じて実行。

## ステータス

> ⚠️ **開発初期段階。** Perch は積極的に開発中であり、まだ本番環境には対応していません。インターフェースや設定は変更される可能性があります。

## クイックスタート

`perch.exe` をダブルクリックし、メニューから `5) Open Web UI (Web UI を開く)` を選択 — Perch はデフォルトの `perch.toml`（`demo` プロセスとトークン `change-me` を含む）を自動生成し、サーバーを起動します。

またはコマンドラインから：

```
# フォアグラウンドで実行（Ctrl-C で停止）
perch run

# Perch を Windows サービスとして登録（管理者権限が必要）
perch install
perch start

# Web UI を開く
http://127.0.0.1:9000
```

デフォルトのトークンは `change-me` です — 実際の使用前には `perch.toml` で変更してください。

## セキュリティ

管理エンドポイントは任意のプロセスを開始・停止できるため、**非常に高い権限**を持ちます。Perch はデフォルトで `127.0.0.1` にバインドし、アクセストークンを要求します。管理ポートを信頼できないネットワークに公開しないでください。

## 比較

| | Perch | NSSM / WinSw | PM2 | Supervisor |
|---|:---:|:---:|:---:|:---:|
| シングルバイナリ | ✅ | ✅ | ❌ (Node が必要) | ❌ (Python が必要) |
| 1 インスタンスで複数プロセスを管理 | ✅ | ❌ (サービスごとに1つ) | ✅ | ✅ |
| Web UI 内蔵 | ✅ | ❌ | ✅ | ✅ |
| ネイティブ Windows サービス | ✅ | ✅ | ⚠️ | ❌ |
| スケジュールタスク | ✅ | ❌ | ✅ | ⚠️ |
| 日次ログローテーション | ✅ | ❌ | ⚠️ | ⚠️ |
| 低メモリ使用量 | ✅ | ✅ | ❌ | ⚠️ |

## ソースからビルド

```
cargo build --release
```

最近の stable Rust ツールチェーンが必要です。

## 提供するスクリーンショットファイル

スクリーンショット画像ファイルを `docs/screenshots/` に配置してください（最初にディレクトリを作成）、以下の正確なファイル名を使用：

| パス | キャプチャ内容 |
|---|---|
| `docs/screenshots/menu.png` | `perch.exe` ダブルクリック後のコンソールウィンドウ。バイリンガルメニュー（インストール/アンインストール/開始/停止/Web UI を開く/終了）を表示。 |
| `docs/screenshots/login.png` | `http://127.0.0.1:9000/` ログインページ。Perch ロゴとトークン入力欄を含む。 |
| `docs/screenshots/processes.png` | `/processes` ダッシュボード。`demo` カードが Running ステータスを表示し、上部ナビバーにロゴを含む。 |
| `docs/screenshots/process-detail.png` | `/process-detail?name=demo` ページ。PID/CPU/メモリ/稼働時間パネルとライブログストリームを含む。 |
| `docs/screenshots/tasks.png` | `/tasks` ページ。少なくとも1行のスケジュールタスク行、上部ナビバーにロゴを含む。 |

ヒント：1280×800 のビューポートでキャプチャ、PNG 形式、ブラウザの枠をクロップ。ファイルを配置すると、上記の画像参照が自動的にレンダリングされます。

## ライセンス

未定
