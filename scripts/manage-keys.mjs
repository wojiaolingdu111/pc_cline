// AI ToReder 授权码管理脚本
// 使用 Vercel KV (Redis) 动态管理授权码，无需重新部署
//
// 用法:
//   node scripts/manage-keys.mjs list                   查看所有授权码
//   node scripts/manage-keys.mjs add <CODE>             添加授权码
//   node scripts/manage-keys.mjs remove <CODE>          删除授权码
//   node scripts/manage-keys.mjs seed                   从 .env.keys 批量导入
//   node scripts/manage-keys.mjs expire <CODE> <DAYS>   设置授权码过期(从今天起)
//
// 前置: Vercel 项目需要先启用 KV (Storage → Create Database → Vercel KV)
// 然后在项目设置添加 KV 环境变量 (自动添加)

const KV_KEY = 'license:keys';

const help = `
用法:
  node scripts/manage-keys.mjs <command> [args]

命令:
  list                             查看所有授权码
  add <CODE> [max_activations]     添加授权码
  remove <CODE>                    删除授权码
  seed                             从 .env.keys 文件批量导入
  expire <CODE> <days>             设置授权码过期天数(从今天起)

示例:
  node scripts/manage-keys.mjs list
  node scripts/manage-keys.mjs add LICENSE-ABCD1234
  node scripts/manage-keys.mjs add LICENSE-ABCD1234 3
  node scripts/manage-keys.mjs remove LICENSE-ABCD1234
  node scripts/manage-keys.mjs seed
  node scripts/manage-keys.mjs expire LICENSE-ABCD1234 30
`;

async function getKv() {
  const { createClient } = await import('@vercel/kv');
  const kv = createClient({
    url: process.env.KV_REST_API_URL,
    token: process.env.KV_REST_API_TOKEN,
  });
  return kv;
}

async function listKeys(kv) {
  const keys = (await kv.get(KV_KEY)) || {};
  console.log('\n📋 授权码列表:\n');
  console.log('  CODE'.padEnd(30), '绑定'.padEnd(14), '激活次数', '过期时间');
  console.log('  ' + '-'.repeat(70));
  for (const [code, record] of Object.entries(keys)) {
    const machine = record.machine_id ? record.machine_id.slice(0, 8) + '…' : '—';
    const expires = record.expires_at
      ? new Date(record.expires_at).toLocaleDateString('zh-CN')
      : '永久';
    console.log(
      `  ${code.padEnd(28)} ${machine.padEnd(12)} ${(record.activation_count || 0) + '/' + (record.max_activations || 1)}  ${expires}`
    );
  }
  console.log(`\n  共 ${Object.keys(keys).length} 个授权码\n`);
}

async function addKey(kv, code, maxActivations) {
  const keys = (await kv.get(KV_KEY)) || {};
  if (keys[code]) {
    console.log(`⚠️  授权码 ${code} 已存在`);
    return;
  }
  keys[code] = {
    max_activations: parseInt(maxActivations) || 1,
  };
  await kv.set(KV_KEY, keys);
  console.log(`✅ 已添加授权码: ${code} (最多激活 ${parseInt(maxActivations) || 1} 台)`);
}

async function removeKey(kv, code) {
  const keys = (await kv.get(KV_KEY)) || {};
  if (!keys[code]) {
    console.log(`❌ 授权码 ${code} 不存在`);
    return;
  }
  delete keys[code];
  await kv.set(KV_KEY, keys);
  console.log(`✅ 已删除授权码: ${code}`);
}

async function expireKey(kv, code, days) {
  const keys = (await kv.get(KV_KEY)) || {};
  if (!keys[code]) {
    console.log(`❌ 授权码 ${code} 不存在`);
    return;
  }
  const expiresAt = Date.now() + parseInt(days) * 86400000;
  keys[code].expires_at = expiresAt;
  await kv.set(KV_KEY, keys);
  console.log(`✅ 授权码 ${code} 已设置为 ${days} 天后过期 (${new Date(expiresAt).toLocaleDateString('zh-CN')})`);
}

async function seedFromEnvFile(kv) {
  const fs = await import('fs');
  const path = await import('path');
  const envPath = path.resolve('.env.keys');
  if (!fs.existsSync(envPath)) {
    console.log('❌ .env.keys 文件不存在');
    return;
  }
  const content = fs.readFileSync(envPath, 'utf-8');
  const match = content.match(/LICENSE_KEYS=(\{.+\})/);
  if (!match) {
    console.log('❌ .env.keys 文件格式错误');
    return;
  }
  const data = JSON.parse(match[1]);
  const existing = (await kv.get(KV_KEY)) || {};
  let count = 0;
  for (const [code, record] of Object.entries(data)) {
    if (!existing[code]) {
      existing[code] = record;
      count++;
    }
  }
  await kv.set(KV_KEY, existing);
  console.log(`✅ 已从 .env.keys 导入 ${count} 个新授权码`);
}

async function main() {
  const cmd = process.argv[2];
  if (!cmd || cmd === '--help' || cmd === '-h') {
    console.log(help);
    return;
  }

  if (!process.env.KV_REST_API_URL || !process.env.KV_REST_API_TOKEN) {
    console.log('❌ 需要 Vercel KV 环境变量');
    console.log('   1. 在 Vercel 项目启用 KV (Storage → Create Database → Vercel KV)');
    console.log('   2. KV 的环境变量会自动注入，或从 Vercel KV 面板复制到本地 .env');
    console.log('\n   本地开发时:');
    console.log('   export KV_REST_API_URL="https://xxx.kv.vercel-storage.com"');
    console.log('   export KV_REST_API_TOKEN="xxx"');
    console.log('   node scripts/manage-keys.mjs list\n');
    process.exit(1);
  }

  const kv = await getKv();

  switch (cmd) {
    case 'list':
      await listKeys(kv);
      break;
    case 'add':
      if (!process.argv[3]) {
        console.log('❌ 请指定授权码: node scripts/manage-keys.mjs add LICENSE-XXXX');
        process.exit(1);
      }
      await addKey(kv, process.argv[3], process.argv[4]);
      break;
    case 'remove':
      if (!process.argv[3]) {
        console.log('❌ 请指定授权码: node scripts/manage-keys.mjs remove LICENSE-XXXX');
        process.exit(1);
      }
      await removeKey(kv, process.argv[3]);
      break;
    case 'seed':
      await seedFromEnvFile(kv);
      break;
    case 'expire':
      if (!process.argv[3] || !process.argv[4]) {
        console.log('❌ 请指定授权码和天数: node scripts/manage-keys.mjs expire LICENSE-XXXX 30');
        process.exit(1);
      }
      await expireKey(kv, process.argv[3], process.argv[4]);
      break;
    default:
      console.log(`❌ 未知命令: ${cmd}\n${help}`);
  }

  await kv.disconnect?.();
}

main().catch(console.error);
