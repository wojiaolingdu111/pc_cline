import { MongoClient } from 'mongodb';

const URI = process.env.MONGODB_URI;
const DB_NAME = process.env.MONGODB_DB || 'pc_clinet';

let cached = null;

export async function connectToDatabase() {
  if (cached) return cached;

  if (!URI) {
    throw new Error('MONGODB_URI 未配置');
  }

  const client = new MongoClient(URI);
  await client.connect();
  const db = client.db(DB_NAME);

  cached = { client, db };
  return cached;
}

export async function getCollection(name = 'licenses') {
  const { db } = await connectToDatabase();
  return db.collection(name);
}
