// AI ToReder 授权码管理脚本 (MongoDB)
// 需要 MONGODB_URI 环境变量
//
// 用法:
//   node scripts/manage-keys.mjs list                   查看所有授权码
//   node scripts/manage-keys.mjs add <CODE>             添加授权码
//   node scripts/manage-keys.mjs remove <CODE>          删除授权码
//   node scripts/manage-keys.mjs seed                   从 .env.keys 批量导入
//   node scripts/manage-keys.mjs expire <CODE> <DAYS>   设置授权码过期(从今天起)
//
// 前置: 需要 MongoDB 连接串
//   export MONGODB_URI="mongodb+srv://user:pass@cluster.mongodb.net"
//   node scripts/manage-keys.mjs list

const { MongoClient } = await import('mongodb');

const URI = process.env.MONGODB_URI;
const DB_NAME = process.env.MONGODB_DB || 'pc_clinet';

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

环境变量:
  MONGODB_URI   MongoDB 连接串 (必填)
  MONGODB_DB    数据库名 (可选, 默认 ai-toreder)
`;

async function getCol() {
  const client = new MongoClient(URI);
  await client.connect();
  const db = client.db(DB_NAME);
  const col = db.collection('licenses');
  return { client, col };
}

async function listKeys(col) {
  const docs = await col.find({}).toArray();
  console.log('\n📋 授权码列表:\n');
  console.log('  CODE'.padEnd(30), '绑定'.padEnd(14), '激活次数', '过期时间');
  console.log('  ' + '-'.repeat(70));
  for (const doc of docs) {
    const machine = doc.machine_id ? doc.machine_id.slice(0, 8) + '…' : '—';
    const expires = doc.expires_at
      ? new Date(doc.expires_at).toLocaleDateString('zh-CN')
      : '永久';
    console.log(
      `  ${doc._id.padEnd(28)} ${machine.padEnd(12)} ${(doc.activation_count || 0) + '/' + (doc.max_activations || 1)}  ${expires}`
    );
  }
  console.log(`\n  共 ${docs.length} 个授权码\n`);
}

async function addKey(col, code, maxActivations) {
  const existing = await col.findOne({ _id: code });
  if (existing) {
    console.log(`⚠️  授权码 ${code} 已存在`);
    return;
  }
  await col.insertOne({
    _id: code,
    max_activations: parseInt(maxActivations) || 1,
    activation_count: 0,
    machine_id: null,
    activated_at: null,
    expires_at: null,
  });
  console.log(`✅ 已添加授权码: ${code} (最多激活 ${parseInt(maxActivations) || 1} 台)`);
}

async function removeKey(col, code) {
  const result = await col.deleteOne({ _id: code });
  if (result.deletedCount === 0) {
    console.log(`❌ 授权码 ${code} 不存在`);
    return;
  }
  console.log(`✅ 已删除授权码: ${code}`);
}

async function expireKey(col, code, days) {
  const expiresAt = Date.now() + parseInt(days) * 86400000;
  const result = await col.updateOne(
    { _id: code },
    { $set: { expires_at: expiresAt } }
  );
  if (result.matchedCount === 0) {
    console.log(`❌ 授权码 ${code} 不存在`);
    return;
  }
  console.log(`✅ 授权码 ${code} 已设置为 ${days} 天后过期 (${new Date(expiresAt).toLocaleDateString('zh-CN')})`);
}

async function seedFromEnvFile(col) {
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
  let count = 0;
  for (const [code, record] of Object.entries(data)) {
    const existing = await col.findOne({ _id: code });
    if (!existing) {
      await col.insertOne({
        _id: code,
        max_activations: record.max_activations || 1,
        activation_count: 0,
        machine_id: null,
        activated_at: null,
        expires_at: null,
      });
      count++;
    }
  }
  console.log(`✅ 已从 .env.keys 导入 ${count} 个新授权码`);
}

async function main() {
  const cmd = process.argv[2];
  if (!cmd || cmd === '--help' || cmd === '-h') {
    console.log(help);
    return;
  }

  if (!URI) {
    console.log('❌ 需要 MongoDB 连接串环境变量');
    console.log('   示例:');
    console.log('   export MONGODB_URI="mongodb+srv://user:pass@cluster.mongodb.net"');
    console.log('   node scripts/manage-keys.mjs list\n');
    process.exit(1);
  }

  const { client, col } = await getCol();

  try {
    switch (cmd) {
      case 'list':
        await listKeys(col);
        break;
      case 'add':
        if (!process.argv[3]) {
          console.log('❌ 请指定授权码: node scripts/manage-keys.mjs add LICENSE-XXXX');
          process.exit(1);
        }
        await addKey(col, process.argv[3], process.argv[4]);
        break;
      case 'remove':
        if (!process.argv[3]) {
          console.log('❌ 请指定授权码: node scripts/manage-keys.mjs remove LICENSE-XXXX');
          process.exit(1);
        }
        await removeKey(col, process.argv[3]);
        break;
      case 'seed':
        await seedFromEnvFile(col);
        break;
      case 'expire':
        if (!process.argv[3] || !process.argv[4]) {
          console.log('❌ 请指定授权码和天数: node scripts/manage-keys.mjs expire LICENSE-XXXX 30');
          process.exit(1);
        }
        await expireKey(col, process.argv[3], process.argv[4]);
        break;
      default:
        console.log(`❌ 未知命令: ${cmd}\n${help}`);
    }
  } finally {
    await client.close();
  }
}

main().catch(console.error);
