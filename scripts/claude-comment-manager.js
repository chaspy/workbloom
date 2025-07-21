#!/usr/bin/env node

/**
 * Claude Comment Manager
 * 
 * このスクリプトはPRの古いClaudeコメントを管理します
 * Usage: node claude-comment-manager.js <pr-number> [options]
 */

const { Octokit } = require('@octokit/rest');

class ClaudeCommentManager {
  constructor(token, owner, repo) {
    this.octokit = new Octokit({ auth: token });
    this.owner = owner;
    this.repo = repo;
  }

  async getClaudeComments(prNumber) {
    try {
      const { data: comments } = await this.octokit.rest.issues.listComments({
        owner: this.owner,
        repo: this.repo,
        issue_number: prNumber,
        per_page: 100
      });

      // Claude コメントを識別
      return comments.filter(comment => {
        const isBot = comment.user.type === 'Bot';
        const hasClaudeSignature = 
          comment.body.includes('Generated with [Claude Code]') || 
          comment.body.includes('Co-Authored-By: Claude') ||
          comment.body.includes('@claude') ||
          comment.user.login.toLowerCase().includes('claude') ||
          comment.user.login.includes('github-actions');
        
        return isBot || hasClaudeSignature;
      });
    } catch (error) {
      console.error(`コメント取得エラー: ${error.message}`);
      throw error;
    }
  }

  async minimizeOldComments(prNumber, keepLatest = 3, olderThanDays = 1) {
    console.log(`PR #${prNumber} のClaudeコメント管理を開始...`);
    
    const claudeComments = await this.getClaudeComments(prNumber);
    console.log(`${claudeComments.length}個のClaudeコメントを発見`);

    if (claudeComments.length <= keepLatest) {
      console.log('最小化する必要があるコメントはありません');
      return;
    }

    // 作成日時順にソート（古い順）
    claudeComments.sort((a, b) => new Date(a.created_at) - new Date(b.created_at));

    // 最新のN個を除いて古いコメントを特定
    const commentsToMinimize = claudeComments.slice(0, -keepLatest);
    
    let minimizedCount = 0;

    for (const comment of commentsToMinimize) {
      const daysSinceCreation = (Date.now() - new Date(comment.created_at).getTime()) / (1000 * 60 * 60 * 24);
      
      if (daysSinceCreation > olderThanDays) {
        // すでに最小化されているかチェック
        if (comment.body.includes('<details>') && comment.body.includes('古いClaudeコメント')) {
          console.log(`コメント ${comment.id} は既に最小化済み`);
          continue;
        }

        try {
          const minimizedBody = `<details>
<summary>🔽 古いClaudeコメント（${new Date(comment.created_at).toLocaleDateString('ja-JP')}）- クリックで展開</summary>

${comment.body}

</details>`;

          await this.octokit.rest.issues.updateComment({
            owner: this.owner,
            repo: this.repo,
            comment_id: comment.id,
            body: minimizedBody
          });

          console.log(`✅ コメント ${comment.id} を最小化しました（${daysSinceCreation.toFixed(1)}日経過）`);
          minimizedCount++;
        } catch (error) {
          console.error(`❌ コメント ${comment.id} の最小化に失敗: ${error.message}`);
        }
      } else {
        console.log(`コメント ${comment.id} はまだ新しいためスキップ（${daysSinceCreation.toFixed(1)}日経過）`);
      }
    }

    console.log(`\n完了: ${minimizedCount}個のコメントを最小化しました`);
  }

  async restoreComments(prNumber) {
    console.log(`PR #${prNumber} のClaudeコメントを復元中...`);
    
    const claudeComments = await this.getClaudeComments(prNumber);
    let restoredCount = 0;

    for (const comment of claudeComments) {
      if (comment.body.includes('<details>') && comment.body.includes('古いClaudeコメント')) {
        try {
          // <details>タグを削除して元のコンテンツを復元
          const originalBody = comment.body
            .replace(/<details>[\s\S]*?<\/summary>\s*/, '')
            .replace('\n</details>', '')
            .trim();

          await this.octokit.rest.issues.updateComment({
            owner: this.owner,
            repo: this.repo,
            comment_id: comment.id,
            body: originalBody
          });

          console.log(`✅ コメント ${comment.id} を復元しました`);
          restoredCount++;
        } catch (error) {
          console.error(`❌ コメント ${comment.id} の復元に失敗: ${error.message}`);
        }
      }
    }

    console.log(`\n完了: ${restoredCount}個のコメントを復元しました`);
  }

  async listClaudeComments(prNumber) {
    console.log(`PR #${prNumber} のClaudeコメント一覧:`);
    
    const claudeComments = await this.getClaudeComments(prNumber);
    
    if (claudeComments.length === 0) {
      console.log('Claudeコメントが見つかりません');
      return;
    }

    claudeComments.forEach((comment, index) => {
      const daysSinceCreation = (Date.now() - new Date(comment.created_at).getTime()) / (1000 * 60 * 60 * 24);
      const isMinimized = comment.body.includes('<details>') && comment.body.includes('古いClaudeコメント');
      const status = isMinimized ? '🔽 最小化済み' : '👁️ 表示中';
      
      console.log(`${index + 1}. ID: ${comment.id}`);
      console.log(`   作成者: ${comment.user.login}`);
      console.log(`   日時: ${new Date(comment.created_at).toLocaleString('ja-JP')}`);
      console.log(`   経過: ${daysSinceCreation.toFixed(1)}日`);
      console.log(`   状態: ${status}`);
      console.log(`   URL: ${comment.html_url}`);
      console.log('');
    });
  }
}

// コマンドライン引数の処理
async function main() {
  const args = process.argv.slice(2);
  
  if (args.length < 1) {
    console.log(`使用方法:
  node claude-comment-manager.js <pr-number> [command] [options]

コマンド:
  minimize (デフォルト) - 古いコメントを最小化
  restore             - 最小化されたコメントを復元
  list               - Claudeコメントの一覧表示

オプション:
  --keep <number>     最新何個のコメントを保持するか (デフォルト: 3)
  --days <number>     何日より古いコメントを対象とするか (デフォルト: 1)

環境変数:
  GITHUB_TOKEN       GitHubアクセストークン
  GITHUB_REPOSITORY  リポジトリ名 (owner/repo形式)

例:
  node claude-comment-manager.js 123
  node claude-comment-manager.js 123 minimize --keep 2 --days 3
  node claude-comment-manager.js 123 restore
  node claude-comment-manager.js 123 list
`);
    process.exit(1);
  }

  const prNumber = parseInt(args[0]);
  const command = args[1] || 'minimize';
  
  // オプションの解析
  let keepLatest = 3;
  let olderThanDays = 1;
  
  for (let i = 2; i < args.length; i += 2) {
    if (args[i] === '--keep' && args[i + 1]) {
      keepLatest = parseInt(args[i + 1]);
    } else if (args[i] === '--days' && args[i + 1]) {
      olderThanDays = parseInt(args[i + 1]);
    }
  }

  const token = process.env.GITHUB_TOKEN;
  const repository = process.env.GITHUB_REPOSITORY || 'chaspy/workbloom';
  
  if (!token) {
    console.error('エラー: GITHUB_TOKEN環境変数が設定されていません');
    process.exit(1);
  }

  const [owner, repo] = repository.split('/');
  if (!owner || !repo) {
    console.error('エラー: GITHUB_REPOSITORY環境変数が正しく設定されていません (owner/repo形式)');
    process.exit(1);
  }

  const manager = new ClaudeCommentManager(token, owner, repo);

  try {
    switch (command) {
      case 'minimize':
        await manager.minimizeOldComments(prNumber, keepLatest, olderThanDays);
        break;
      case 'restore':
        await manager.restoreComments(prNumber);
        break;
      case 'list':
        await manager.listClaudeComments(prNumber);
        break;
      default:
        console.error(`未知のコマンド: ${command}`);
        process.exit(1);
    }
  } catch (error) {
    console.error(`エラーが発生しました: ${error.message}`);
    process.exit(1);
  }
}

if (require.main === module) {
  main();
}

module.exports = ClaudeCommentManager;