/**
 * Flackle permission bits. Values from the web client (`module 4374`), stored
 * as 33-bit decimal strings in API payloads.
 */
export const Permission = {
  ViewChannel: 1n << 0n,
  SendMessages: 1n << 1n,
  AttachFiles: 1n << 2n,
  ManageMessages: 1n << 3n,
  MentionEveryone: 1n << 4n,
  CreateInvite: 1n << 5n,
  KickMembers: 1n << 6n,
  BanMembers: 1n << 7n,
  ManageChannels: 1n << 8n,
  ManageRoles: 1n << 9n,
  ManageServer: 1n << 10n,
  ChangeNickname: 1n << 11n,
  ManageNicknames: 1n << 12n,
  Administrator: 1n << 13n,
  AddReactions: 1n << 14n,
  Connect: 1n << 15n,
  Speak: 1n << 16n,
  MuteMembers: 1n << 17n,
  MoveMembers: 1n << 18n,
  DeafenMembers: 1n << 20n,
  UseVideo: 1n << 21n,
  ManageMembers: 1n << 22n,
  ViewGeneralStats: 1n << 24n,
  ViewElevatedStats: 1n << 25n,
  BypassSlowmode: 1n << 26n,
  TimeoutMembers: 1n << 27n,
  BulkDelete: 1n << 28n,
  ViewAuditLog: 1n << 29n,
  ManagePermissions: 1n << 30n,
  PinMessages: 1n << 31n,
  SaveMedia: 1n << 32n,
} as const;

export type PermissionName = keyof typeof Permission;

export const ALL_PERMISSIONS = (1n << 33n) - 1n;
export const DEFAULT_MEMBER_PERMISSIONS =
  Permission.ViewChannel |
  Permission.SendMessages |
  Permission.AttachFiles |
  Permission.AddReactions |
  Permission.CreateInvite;

export function toBits(value: string | number | bigint | null | undefined): bigint {
  if (value === null || value === undefined || value === "" || value === "0") return 0n;
  try {
    return BigInt(value);
  } catch {
    return 0n;
  }
}

export function hasPermission(
  value: string | number | bigint | null | undefined,
  permission: PermissionName,
): boolean {
  const bits = toBits(value);
  if ((bits & Permission.Administrator) !== 0n) return true;
  return (bits & Permission[permission]) === Permission[permission];
}

export function permissionList(value: string | number | bigint | null | undefined): PermissionName[] {
  const bits = toBits(value);
  return (Object.keys(Permission) as PermissionName[]).filter(
    (name) => (bits & Permission[name]) === Permission[name],
  );
}
