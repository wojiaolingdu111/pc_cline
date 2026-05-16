import { getCollection } from '../lib/mongo.js';

export async function withAuth(req, res) {
  const password = process.env.ADMIN_PASSWORD;
  if (!password) {
    res.status(500).json({ error: 'ADMIN_PASSWORD 未配置' });
    return false;
  }

  const token = req.headers.authorization?.replace('Bearer ', '');
  if (token !== password) {
    res.status(401).json({ error: '密码错误' });
    return false;
  }
  return true;
}

export async function loadKeys() {
  const col = await getCollection();
  const docs = await col.find({}).toArray();
  const map = {};
  for (const doc of docs) {
    const { _id, ...rest } = doc;
    map[_id] = { ...rest };
  }
  return map;
}

export async function saveKeys(keys) {
  const col = await getCollection();
  await col.deleteMany({});
  if (Object.keys(keys).length > 0) {
    const docs = Object.entries(keys).map(([code, data]) => ({ _id: code, ...data }));
    await col.insertMany(docs);
  }
}
