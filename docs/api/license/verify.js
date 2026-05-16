// Vercel API: 验证授权码
// 支持 MongoDB 和 环境变量(LICENSE_KEYS) 两种存储方式

import { getCollection } from '../lib/mongo.js';

export default async function handler(req, res) {
  if (req.method !== 'POST') {
    return res.status(405).json({ valid: false, message: 'Method not allowed' });
  }

  const { license_key, machine_id } = req.body;
  if (!license_key) {
    return res.status(400).json({ valid: false, message: '缺少授权码' });
  }

  try {
    const record = await loadLicenseKey(license_key);

    if (!record) {
      return res.status(200).json({ valid: false, message: '授权码无效' });
    }

    if (record.expires_at && Date.now() > record.expires_at) {
      return res.status(200).json({ valid: false, message: '授权码已过期' });
    }

    // Check machine binding
    if (record.machine_id && record.machine_id !== machine_id) {
      return res.status(200).json({ valid: false, message: '授权码已被其他设备绑定' });
    }

    return res.status(200).json({
      valid: true,
      message: '授权码有效',
      expires_at: record.expires_at || null,
    });
  } catch (error) {
    console.error('verify error:', error);
    return res.status(500).json({ valid: false, message: '服务器内部错误' });
  }
}

async function loadLicenseKey(code) {
  // 优先使用 MongoDB
  try {
    const col = await getCollection();
    const doc = await col.findOne({ _id: code });
    if (doc) return doc;
  } catch (e) {
    console.warn('MongoDB 不可用, 回退到环境变量:', e.message);
  }

  // 回退到环境变量
  if (process.env.LICENSE_KEYS) {
    try {
      const keys = JSON.parse(process.env.LICENSE_KEYS);
      return keys[code] || null;
    } catch { /* ignore */ }
  }

  return null;
}
