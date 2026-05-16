// Vercel API: 激活授权码并绑定机器
// 支持 MongoDB 和 LICENSE_KEYS 环境变量 (环境变量模式不支持回写)
//
// 绑定流程:
//   1. 验证授权码是否存在且未过期
//   2. 检查激活次数是否超限
//   3. 绑定 machine_id → 其他设备不可用此码
//   4. 返回激活结果

import { getCollection } from '../lib/mongo.js';

export default async function handler(req, res) {
  if (req.method !== 'POST') {
    return res.status(405).json({ valid: false, message: 'Method not allowed' });
  }

  const { license_key, machine_id } = req.body;
  if (!license_key) {
    return res.status(400).json({ valid: false, message: '缺少授权码' });
  }
  if (!machine_id) {
    return res.status(400).json({ valid: false, message: '缺少机器标识' });
  }

  try {
    const record = await loadLicenseKey(license_key);

    if (!record) {
      return res.status(200).json({ valid: false, message: '授权码无效' });
    }

    if (record.expires_at && Date.now() > record.expires_at) {
      return res.status(200).json({ valid: false, message: '授权码已过期' });
    }

    // 已绑定其他机器
    if (record.machine_id && record.machine_id !== machine_id) {
      return res.status(200).json({ valid: false, message: '授权码已被其他设备绑定' });
    }

    // 首次激活：绑定机器
    if (!record.machine_id) {
      const maxActivations = record.max_activations || 1;
      if ((record.activation_count || 0) >= maxActivations) {
        return res.status(200).json({ valid: false, message: '授权码激活次数已用完' });
      }

      const updated = await activateLicenseKey(license_key, machine_id);
      if (!updated) {
        return res.status(200).json({ valid: false, message: '授权码激活失败' });
      }
    }

    return res.status(200).json({
      valid: true,
      message: '激活成功',
      expires_at: record.expires_at || null,
    });
  } catch (error) {
    console.error('activate error:', error);
    return res.status(500).json({ valid: false, message: '服务器内部错误' });
  }
}

async function loadLicenseKey(code) {
  try {
    const col = await getCollection();
    const doc = await col.findOne({ _id: code });
    if (doc) return doc;
  } catch (e) {
    console.warn('MongoDB 不可用, 回退到环境变量:', e.message);
  }

  if (process.env.LICENSE_KEYS) {
    try {
      const keys = JSON.parse(process.env.LICENSE_KEYS);
      return keys[code] || null;
    } catch { /* ignore */ }
  }

  return null;
}

async function activateLicenseKey(code, machineId) {
  try {
    const col = await getCollection();
    const result = await col.findOneAndUpdate(
      { _id: code, machine_id: null },
      {
        $set: { machine_id: machineId, activated_at: Date.now() },
        $inc: { activation_count: 1 },
      },
      { returnDocument: 'after' }
    );
    return result;
  } catch (e) {
    console.error('MongoDB 更新失败:', e.message);
    return null;
  }
}
