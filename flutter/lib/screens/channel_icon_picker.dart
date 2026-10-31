import 'package:flutter/material.dart';

import '../core/theme.dart';
import '../ui/common.dart';
import '../ui/channel_icons.dart';
import '../ui/icons.dart';

/// Opens Flackle's channel icon picker. Returns `null` when the user cancels,
/// an empty string when they choose no icon, or an icon name.
Future<String?> showChannelIconPicker(
  BuildContext context, {
  String? selected,
}) {
  return showModalBottomSheet<String>(
    context: context,
    isScrollControlled: true,
    backgroundColor: Colors.transparent,
    builder: (context) => _ChannelIconPicker(selected: selected),
  );
}

class _ChannelIconPicker extends StatefulWidget {
  const _ChannelIconPicker({this.selected});

  final String? selected;

  @override
  State<_ChannelIconPicker> createState() => _ChannelIconPickerState();
}

class _ChannelIconPickerState extends State<_ChannelIconPicker> {
  final TextEditingController _search = TextEditingController();
  String _query = '';

  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final c = context.c;
    final query = _query.trim().toLowerCase();
    final icons = kChannelIconNames
        .where((name) =>
            query.isEmpty ||
            name.toLowerCase().contains(query) ||
            channelIconLabel(name).toLowerCase().contains(query))
        .toList();
    return Padding(
      padding: EdgeInsets.only(bottom: MediaQuery.viewInsetsOf(context).bottom),
      child: Container(
        height: MediaQuery.sizeOf(context).height * 0.72,
        decoration: BoxDecoration(
          color: c.panel,
          borderRadius: const BorderRadius.vertical(top: Radius.circular(18)),
          border: Border.all(color: c.line),
        ),
        clipBehavior: Clip.antiAlias,
        child: Column(
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(18, 14, 12, 8),
              child: Row(
                children: [
                  ChannelGlyph(
                    name: widget.selected,
                    color: null,
                    type: 'text',
                    size: 18,
                  ),
                  const SizedBox(width: 9),
                  Expanded(
                    child: Text('Channel icon',
                        style: context.display(19, weight: FontWeight.w600)),
                  ),
                  TextButton(
                    onPressed: () => Navigator.of(context).pop(''),
                    child: const Text('No icon'),
                  ),
                  IconButton(
                    onPressed: () => Navigator.of(context).pop(),
                    icon: const Icon(Icons.close_rounded, size: 18),
                  ),
                ],
              ),
            ),
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 0, 16, 10),
              child: TextField(
                controller: _search,
                onChanged: (value) => setState(() => _query = value),
                decoration: const InputDecoration(
                  hintText: 'Search icons',
                  prefixIcon: Icon(Icons.search_rounded, size: 18),
                  isDense: true,
                ),
              ),
            ),
            Expanded(
              child: icons.isEmpty
                  ? Center(
                      child: Text('No icons match that search.',
                          style: TextStyle(color: c.muted, fontSize: 13)),
                    )
                  : GridView.builder(
                      padding: const EdgeInsets.fromLTRB(14, 2, 14, 18),
                      gridDelegate:
                          const SliverGridDelegateWithMaxCrossAxisExtent(
                        maxCrossAxisExtent: 54,
                        mainAxisExtent: 50,
                        crossAxisSpacing: 6,
                        mainAxisSpacing: 6,
                      ),
                      itemCount: icons.length,
                      itemBuilder: (context, index) {
                        final name = icons[index];
                        final active = name == widget.selected;
                        return Tooltip(
                          message: channelIconLabel(name),
                          child: InkWell(
                            borderRadius: BorderRadius.circular(9),
                            onTap: () => Navigator.of(context).pop(name),
                            child: Container(
                              decoration: BoxDecoration(
                                color: active
                                    ? c.accent.withValues(alpha: 0.16)
                                    : c.input,
                                borderRadius: BorderRadius.circular(9),
                                border: Border.all(
                                  color: active ? c.accent : c.line,
                                  width: active ? 1.5 : 1,
                                ),
                              ),
                              child: Center(
                                child: AppIcon(
                                  'hash',
                                  asset: name,
                                  color: active ? c.accent : c.textDim,
                                  size: 20,
                                ),
                              ),
                            ),
                          ),
                        );
                      },
                    ),
            ),
          ],
        ),
      ),
    );
  }
}
