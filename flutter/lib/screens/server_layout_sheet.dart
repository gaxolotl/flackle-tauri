import 'dart:async';

import 'package:flutter/material.dart';

import '../core/models.dart';
import '../core/state.dart';
import '../core/theme.dart';
import '../ui/common.dart';
import '../ui/emoji.dart';
import '../ui/icons.dart';

const _folderColors = [
  '#95a5a6',
  '#8b6ce8',
  '#2fbd9f',
  '#4a9be8',
  '#e05ea8',
  '#e0a13d',
  '#ef5252',
];

Future<void> showServerLayoutSheet(BuildContext context) async {
  final state = FlackleScope.read(context);
  await showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    backgroundColor: Colors.transparent,
    builder: (context) => AnimatedBuilder(
      animation: state,
      builder: (context, _) => _ServerLayoutSheet(state: state),
    ),
  );
}

class _ServerLayoutSheet extends StatelessWidget {
  const _ServerLayoutSheet({required this.state});

  final AppState state;

  @override
  Widget build(BuildContext context) {
    final c = context.c;
    final entries = state.serverRail;
    return Container(
      height: MediaQuery.sizeOf(context).height * 0.84,
      decoration: BoxDecoration(
        color: c.panel,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(18)),
        border: Border.all(color: c.line),
      ),
      clipBehavior: Clip.antiAlias,
      child: Column(
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(18, 14, 10, 10),
            child: Row(
              children: [
                AppIcon('sliders', size: 17, color: c.accent),
                const SizedBox(width: 8),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('Server layout',
                          style: context.display(19, weight: FontWeight.w600)),
                      Text(
                          'Drag to reorder. Folders keep related servers together.',
                          style: context.label(11.5, color: c.muted)),
                    ],
                  ),
                ),
                TextButton.icon(
                  onPressed: () => showServerFolderDialog(context),
                  icon: const Icon(Icons.create_new_folder_outlined, size: 16),
                  label: const Text('New folder'),
                ),
                IconButton(
                  onPressed: () => Navigator.of(context).pop(),
                  icon: const Icon(Icons.close_rounded, size: 18),
                ),
              ],
            ),
          ),
          Divider(height: 1, color: c.line),
          Expanded(
            child: entries.isEmpty
                ? Center(
                    child: Text('No servers yet.',
                        style: TextStyle(color: c.muted, fontSize: 13)),
                  )
                : ReorderableListView.builder(
                    padding: const EdgeInsets.fromLTRB(14, 10, 14, 24),
                    buildDefaultDragHandles: false,
                    itemCount: entries.length,
                    onReorder: (oldIndex, newIndex) {
                      if (newIndex > oldIndex) newIndex -= 1;
                      final next = [...entries];
                      final item = next.removeAt(oldIndex);
                      next.insert(newIndex, item);
                      unawaited(state.reorderServerRail(next).then((ok) {
                        if (!ok && context.mounted) {
                          showAppToast(context, 'Could not save the layout.');
                        }
                      }));
                    },
                    proxyDecorator: (child, index, animation) =>
                        Material(color: Colors.transparent, child: child),
                    itemBuilder: (context, index) {
                      final entry = entries[index];
                      return entry.isFolder
                          ? _folderEntry(context, entry, index)
                          : _serverEntry(context, entry, index);
                    },
                  ),
          ),
        ],
      ),
    );
  }

  Widget _folderEntry(BuildContext context, ServerRailEntry entry, int index) {
    final c = context.c;
    final folder = entry.folder!;
    return Container(
      key: ValueKey(entry.key),
      margin: const EdgeInsets.only(bottom: 8),
      decoration: BoxDecoration(
        color: c.panel2,
        borderRadius: BorderRadius.circular(11),
        border: Border.all(color: c.line),
      ),
      child: Column(
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(4, 5, 4, 5),
            child: Row(
              children: [
                ReorderableDragStartListener(
                  index: index,
                  child: Padding(
                    padding: const EdgeInsets.all(6),
                    child: Icon(Icons.drag_indicator_rounded,
                        size: 18, color: c.muted),
                  ),
                ),
                _folderIcon(context, folder),
                const SizedBox(width: 9),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      EmojiText(folder.name,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: context.label(13, weight: FontWeight.w600)),
                      Text(
                          entry.servers.isEmpty
                              ? 'Empty folder'
                              : '${entry.servers.length} server${entry.servers.length == 1 ? '' : 's'}',
                          style: context.label(10.5, color: c.muted)),
                    ],
                  ),
                ),
                PopupMenuButton<String>(
                  tooltip: 'Folder options',
                  icon:
                      Icon(Icons.more_horiz_rounded, size: 18, color: c.muted),
                  onSelected: (value) {
                    if (value == 'edit') {
                      unawaited(
                          showServerFolderDialog(context, folder: folder));
                    } else if (value == 'delete') {
                      unawaited(_deleteFolder(context, folder));
                    }
                  },
                  itemBuilder: (context) => const [
                    PopupMenuItem(value: 'edit', child: Text('Rename folder')),
                    PopupMenuItem(
                      value: 'delete',
                      child: Text('Delete folder',
                          style: TextStyle(color: Color(0xFFEF5252))),
                    ),
                  ],
                ),
              ],
            ),
          ),
          if (entry.servers.isNotEmpty) ...[
            Divider(height: 1, color: c.line),
            for (final server in entry.servers)
              Padding(
                padding: const EdgeInsets.fromLTRB(38, 2, 4, 2),
                child: Row(
                  children: [
                    AppAvatar(
                      url: server.iconUrl,
                      name: server.name,
                      size: 28,
                      square: true,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: EmojiText(server.name,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: context.label(12, color: c.textDim)),
                    ),
                    IconButton(
                      tooltip: 'Remove from folder',
                      visualDensity: VisualDensity.compact,
                      onPressed: () =>
                          unawaited(state.moveServerToFolder(server.id, null)),
                      icon: Icon(Icons.close_rounded, size: 15, color: c.muted),
                    ),
                  ],
                ),
              ),
          ],
        ],
      ),
    );
  }

  Widget _serverEntry(BuildContext context, ServerRailEntry entry, int index) {
    final c = context.c;
    final server = entry.server!;
    final folder = state.folderById(state.serverFolderId(server.id) ?? '');
    return Container(
      key: ValueKey(entry.key),
      margin: const EdgeInsets.only(bottom: 6),
      padding: const EdgeInsets.fromLTRB(4, 5, 4, 5),
      decoration: BoxDecoration(
        color: c.panel2,
        borderRadius: BorderRadius.circular(11),
        border: Border.all(color: c.line),
      ),
      child: Row(
        children: [
          ReorderableDragStartListener(
            index: index,
            child: Padding(
              padding: const EdgeInsets.all(6),
              child:
                  Icon(Icons.drag_indicator_rounded, size: 18, color: c.muted),
            ),
          ),
          AppAvatar(
            url: server.iconUrl,
            name: server.name,
            size: 32,
            square: true,
          ),
          const SizedBox(width: 9),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                EmojiText(server.name,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: context.label(13, weight: FontWeight.w600)),
                Text(folder == null ? 'No folder' : 'In ${folder.name}',
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: context.label(10.5, color: c.muted)),
              ],
            ),
          ),
          PopupMenuButton<String>(
            tooltip: 'Server layout',
            icon: Icon(Icons.more_horiz_rounded, size: 18, color: c.muted),
            onSelected: (value) =>
                unawaited(_serverAction(context, server, value)),
            itemBuilder: (context) => [
              for (final folder in state.folders)
                PopupMenuItem(
                  value: 'move:${folder.id}',
                  child: Text('Move to ${folder.name}'),
                ),
              if (state.serverFolderId(server.id) != null)
                const PopupMenuItem(
                    value: 'move:', child: Text('Remove from folder')),
              const PopupMenuDivider(),
              const PopupMenuItem(
                value: 'new',
                child: Text('Create a folder for this server'),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Future<void> _serverAction(
      BuildContext context, Server server, String value) async {
    if (value == 'new') {
      await showServerFolderDialog(context, serverId: server.id);
      return;
    }
    if (value.startsWith('move:')) {
      final folderId = value.substring(5);
      final ok = await state.moveServerToFolder(
          server.id, folderId.isEmpty ? null : folderId);
      if (!ok && context.mounted) {
        showAppToast(context, 'Could not move that server.');
      }
    }
  }

  Future<void> _deleteFolder(BuildContext context, ServerFolder folder) async {
    final confirmed = await confirmAction(
      context,
      title: 'Delete ${folder.name}?',
      body: 'Its servers stay in your list, outside the folder.',
      confirmLabel: 'Delete folder',
      danger: true,
    );
    if (!confirmed) return;
    final ok = await state.deleteServerFolder(folder.id);
    if (!ok && context.mounted) {
      showAppToast(context, 'Could not delete that folder.');
    }
  }

  Widget _folderIcon(BuildContext context, ServerFolder folder) {
    final c = context.c;
    final color = _hexColor(folder.color) ?? c.muted;
    return Container(
      width: 34,
      height: 34,
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.16),
        borderRadius: BorderRadius.circular(9),
        border: Border.all(color: color.withValues(alpha: 0.35)),
      ),
      alignment: Alignment.center,
      child: folder.emoji?.isNotEmpty == true
          ? Text(folder.emoji!, style: const TextStyle(fontSize: 17))
          : Icon(Icons.folder_rounded, size: 19, color: color),
    );
  }
}

Future<void> showServerFolderDialog(
  BuildContext context, {
  ServerFolder? folder,
  String? serverId,
}) async {
  final state = FlackleScope.read(context);
  final name = TextEditingController(text: folder?.name ?? '');
  final emoji = TextEditingController(text: folder?.emoji ?? '');
  String? color = folder?.color;
  String? error;
  var busy = false;

  await showDialog<void>(
    context: context,
    builder: (context) => StatefulBuilder(
      builder: (context, setDialogState) {
        final c = context.c;
        return AlertDialog(
          title: Text(folder == null ? 'New folder' : 'Edit folder'),
          content: SizedBox(
            width: 380,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                TextField(
                  controller: name,
                  autofocus: true,
                  decoration: const InputDecoration(labelText: 'Folder name'),
                ),
                const SizedBox(height: 12),
                TextField(
                  controller: emoji,
                  maxLength: 8,
                  decoration: const InputDecoration(
                    labelText: 'Emoji (optional)',
                    helperText:
                        'Shown on the folder tile instead of server icons.',
                  ),
                ),
                const SizedBox(height: 4),
                Text('Color',
                    style: context.label(11,
                        weight: FontWeight.w600, color: c.muted)),
                const SizedBox(height: 7),
                Wrap(
                  spacing: 7,
                  runSpacing: 7,
                  children: [
                    _folderColorDot(context, null, color, (value) {
                      setDialogState(() => color = value);
                    }),
                    for (final hex in _folderColors)
                      _folderColorDot(context, hex, color, (value) {
                        setDialogState(() => color = value);
                      }),
                  ],
                ),
                if (error != null) ...[
                  const SizedBox(height: 12),
                  Text(error!, style: TextStyle(color: c.danger, fontSize: 12)),
                ],
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: busy ? null : () => Navigator.of(context).pop(),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: busy
                  ? null
                  : () async {
                      final trimmed = name.text.trim();
                      if (trimmed.isEmpty) {
                        setDialogState(() => error = 'Give the folder a name.');
                        return;
                      }
                      setDialogState(() {
                        busy = true;
                        error = null;
                      });
                      final trimmedEmoji = emoji.text.trim();
                      if (folder == null) {
                        final created = await state.createServerFolder(
                          trimmed,
                          color: color,
                          emoji: trimmedEmoji.isEmpty ? null : trimmedEmoji,
                          serverIds: serverId == null ? const [] : [serverId],
                        );
                        if (created == null) {
                          if (context.mounted) {
                            setDialogState(() {
                              busy = false;
                              error = 'Could not create the folder.';
                            });
                          }
                          return;
                        }
                      } else {
                        final ok = await state.updateServerFolder(
                          folder.id,
                          name: trimmed,
                          color: color ?? '',
                          emoji: trimmedEmoji,
                        );
                        if (!ok) {
                          if (context.mounted) {
                            setDialogState(() {
                              busy = false;
                              error = 'Could not save the folder.';
                            });
                          }
                          return;
                        }
                      }
                      if (context.mounted) Navigator.of(context).pop();
                    },
              child: busy
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(
                          strokeWidth: 2, color: Colors.white),
                    )
                  : Text(folder == null ? 'Create' : 'Save'),
            ),
          ],
        );
      },
    ),
  );
  name.dispose();
  emoji.dispose();
}

Widget _folderColorDot(
  BuildContext context,
  String? hex,
  String? selected,
  ValueChanged<String?> onTap,
) {
  final c = context.c;
  final active = selected == hex;
  final color = _hexColor(hex) ?? c.muted;
  return InkWell(
    borderRadius: BorderRadius.circular(99),
    onTap: () => onTap(hex),
    child: Container(
      width: 26,
      height: 26,
      decoration: BoxDecoration(
        color: hex == null ? c.input : color,
        shape: BoxShape.circle,
        border: Border.all(
          color: active ? c.text : c.lineStrong,
          width: active ? 2 : 1,
        ),
      ),
      child: hex == null
          ? Icon(Icons.block_rounded, size: 13, color: c.muted)
          : null,
    ),
  );
}

Color? _hexColor(String? value) {
  if (value == null || value.length != 7 || !value.startsWith('#')) return null;
  final parsed = int.tryParse('FF${value.substring(1)}', radix: 16);
  return parsed == null ? null : Color(parsed);
}
