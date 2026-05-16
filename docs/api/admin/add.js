const { withAuth, loadKeys, saveKeys } = await import('./lib.js');

export default async function handler(req, res) {
  if (!(await withAuth(req, res))) return;
  if (req.method !== 'POST') return res.status(405).json({ error: 'Method not allowed' });

  const { code, max_activations, expires_in_days } = req.body || {};
  if (!code) return res.status(400).json({ error: '缺少授权码' });

  const keys = await loadKeys();
  if (keys[code]) return res.status(400).json({ error: '授权码已存在' });

  keys[code] = {
    max_activations: parseInt(max_activations) || 1,
    expires_at: expires_in_days ? Date.now() + parseInt(expires_in_days) * 86400000 : null,
  };

  await saveKeys(keys);
  return res.status(200).json({ success: true, code });
}
