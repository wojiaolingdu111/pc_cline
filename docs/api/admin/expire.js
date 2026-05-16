const { withAuth, loadKeys, saveKeys } = await import('./lib.js');

export default async function handler(req, res) {
  if (!(await withAuth(req, res))) return;
  if (req.method !== 'POST') return res.status(405).json({ error: 'Method not allowed' });

  const { code, days } = req.body || {};
  if (!code) return res.status(400).json({ error: '缺少授权码' });
  if (days === undefined) return res.status(400).json({ error: '缺少天数' });

  const keys = await loadKeys();
  if (!keys[code]) return res.status(404).json({ error: '授权码不存在' });

  if (days === 0) {
    keys[code].expires_at = null;
  } else {
    keys[code].expires_at = Date.now() + parseInt(days) * 86400000;
  }

  await saveKeys(keys);
  return res.status(200).json({ success: true, expires_at: keys[code].expires_at });
}
