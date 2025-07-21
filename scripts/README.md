# Claude Comment Manager

ClaudeのPRレビューコメントが蓄積されることを防ぐため、古いコメントを自動的に最小化するツールです。

## 機能

- **自動最小化**: 古いClaudeコメントを`<details>`タグで折りたたんで最小化
- **自動実行**: GitHub Actionsで自動実行（PR作成・更新時）
- **手動実行**: コマンドラインからの手動実行もサポート
- **復元機能**: 最小化したコメントを元に戻すことも可能
- **一覧表示**: PRのClaudeコメント状況を確認

## GitHub Actions（自動実行）

`.github/workflows/claude-comment-management.yml`が自動的に以下の場合に実行されます：

- PR作成時
- PRへのプッシュ時
- PRコメント作成時

### 設定

デフォルトでは以下の設定で動作します：
- 最新3つのClaudeコメントは表示のまま保持
- 1日以上古いコメントを最小化対象とする

## 手動実行

より細かい制御が必要な場合は、Node.jsスクリプトを直接実行できます。

### セットアップ

```bash
cd scripts
npm install
```

### 使用方法

#### 環境変数の設定

```bash
export GITHUB_TOKEN="your_github_token"
export GITHUB_REPOSITORY="owner/repo"  # 例: chaspy/workbloom
```

#### 基本的な使用方法

```bash
# PR #123の古いClaudeコメントを最小化
node claude-comment-manager.js 123

# 最新2つのコメントを保持し、3日以上古いコメントを最小化
node claude-comment-manager.js 123 minimize --keep 2 --days 3

# 最小化されたコメントを復元
node claude-comment-manager.js 123 restore

# Claudeコメントの一覧表示
node claude-comment-manager.js 123 list
```

### コマンドオプション

| オプション | 説明 | デフォルト |
|-----------|------|-----------|
| `--keep <number>` | 最新何個のコメントを保持するか | 3 |
| `--days <number>` | 何日より古いコメントを対象とするか | 1 |

### 使用例

```bash
# 例1: デフォルト設定で最小化
node claude-comment-manager.js 123

# 例2: 最新1つだけ保持し、即座に最小化
node claude-comment-manager.js 123 minimize --keep 1 --days 0

# 例3: 1週間以上古いコメントのみ最小化
node claude-comment-manager.js 123 minimize --days 7

# 例4: 状況確認
node claude-comment-manager.js 123 list
```

## 動作の仕組み

1. **Claude コメントの識別**
   - Bot ユーザーによるコメント
   - `Generated with [Claude Code]` を含むコメント
   - `Co-Authored-By: Claude` を含むコメント
   - `@claude` を含むコメント
   - ユーザー名に `claude` を含むコメント

2. **最小化処理**
   - 指定された日数より古いコメントを対象
   - 最新N個のコメントは保持
   - `<details>` タグで折りたたみ
   - 作成日時を日本語で表示

3. **復元処理**
   - `<details>` タグを削除
   - 元のコメント内容を復元

## セキュリティ

- GitHub トークンには `pull-requests:write` と `issues:write` 権限が必要
- プライベートリポジトリでも動作
- コメントの削除は行わず、最小化のみ実行

## トラブルシューティング

### よくあるエラー

1. **権限エラー**
   ```
   Error: Resource not accessible by integration
   ```
   → GitHub トークンの権限を確認してください

2. **PR が見つからない**
   ```
   Error: Not Found
   ```
   → PR番号とリポジトリ名を確認してください

3. **環境変数未設定**
   ```
   Error: GITHUB_TOKEN環境変数が設定されていません
   ```
   → 必要な環境変数を設定してください

### ログの確認

GitHub Actions の実行ログでスクリプトの動作を確認できます：
1. リポジトリの "Actions" タブを開く
2. "Claude Comment Management" ワークフローを選択
3. 実行ログを確認

## カスタマイズ

必要に応じて以下をカスタマイズできます：

- `.github/workflows/claude-comment-management.yml` - ワークフローの設定
- `claude-comment-manager.js` - スクリプトのロジック
- Claude コメントの識別条件
- 最小化の条件（日数、保持数など）