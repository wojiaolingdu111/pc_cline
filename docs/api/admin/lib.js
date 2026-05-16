const KV_KEY = 'license:keys';

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
  if (process.env.KV_REST_API_URL && process.env.KV_REST_API_TOKEN) {
    const { get } = await import('@vercel/kv');
    return (await get(KV_KEY)) || {};
  }
  return {};
}

export async function saveKeys(keys) {
  if (process.env.KV_REST_API_URL && process.env.KV_REST_API_TOKEN) {
    const { set } = await import('@vercel/kv');
    await set(KV_KEY, keys);
  }
}
