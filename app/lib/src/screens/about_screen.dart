import 'dart:io';

import 'package:flutter/material.dart';

import '../app_state.dart';

/// 项目主页与更新地址（GitHub）。
const kProjectHome = 'https://github.com/nekwken/mhy-QRscanner';
const kProjectReleases = 'https://github.com/nekwken/mhy-QRscanner/releases';

/// 关于页只回答三件事：这是哪个版本、数据在哪、去哪里看源码与更新。
class AboutScreen extends StatelessWidget {
  const AboutScreen({super.key, required this.state});

  final AppState state;

  void _open(String url) {
    Process.run('rundll32', ['url.dll,FileProtocolHandler', url]);
  }

  @override
  Widget build(BuildContext context) {
    final info = state.info;
    return ListView(
      padding: const EdgeInsets.all(24),
      children: [
        Text('关于', style: Theme.of(context).textTheme.headlineSmall),
        const SizedBox(height: 16),
        if (info == null)
          const Text('核心尚未初始化。')
        else ...[
          _Row(label: '版本', value: info.coreVersion),
          _Row(label: '数据目录', value: info.dataDir),
        ],
        const SizedBox(height: 8),
        _LinkRow(
          label: '项目主页',
          value: kProjectHome,
          onTap: () => _open(kProjectHome),
        ),
        _LinkRow(
          label: '更新下载',
          value: kProjectReleases,
          onTap: () => _open(kProjectReleases),
        ),
      ],
    );
  }
}

class _Row extends StatelessWidget {
  const _Row({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(width: 130, child: Text(label)),
          Expanded(
            child: SelectableText(
              value,
              style: const TextStyle(fontFamily: 'monospace'),
            ),
          ),
        ],
      ),
    );
  }
}

class _LinkRow extends StatelessWidget {
  const _LinkRow({
    required this.label,
    required this.value,
    required this.onTap,
  });

  final String label;
  final String value;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(width: 130, child: Text(label)),
          Expanded(
            child: InkWell(
              onTap: onTap,
              child: Padding(
                padding: const EdgeInsets.symmetric(vertical: 2),
                child: Row(
                  children: [
                    Flexible(
                      child: Text(
                        value,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                          fontFamily: 'monospace',
                          color: scheme.primary,
                          decoration: TextDecoration.underline,
                        ),
                      ),
                    ),
                    const SizedBox(width: 6),
                    Icon(Icons.open_in_new, size: 14, color: scheme.primary),
                  ],
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
