import React, { useState } from "react";
import { ShieldCheck, UserCheck, UserPlus } from "lucide-react";

interface CreatorHeaderProps {
  name: string;
  avatar?: string;
  verified?: boolean;
  packageCount?: number;
  totalDownloads?: number;
}

export const CreatorHeader: React.FC<CreatorHeaderProps> = ({
  name,
  avatar,
  verified,
  packageCount,
  totalDownloads,
}) => {
  const [following, setFollowing] = useState(false);

  return (
    <div className="flex items-center justify-between gap-3 p-2.5 rounded-xl bg-[var(--rz-surface)] border border-[var(--rz-border-subtle)] my-3">
      <div className="flex items-center gap-2.5 min-w-0">
        {avatar ? (
          <img
            src={avatar}
            alt={name}
            className="w-9 h-9 rounded-full object-cover border border-[var(--rz-border-subtle)] shrink-0"
            onError={(e) => {
              (e.target as HTMLElement).style.display = "none";
            }}
          />
        ) : (
          <div className="w-9 h-9 rounded-full bg-[var(--rz-surface-elevated)] border border-[var(--rz-border-subtle)] flex items-center justify-center font-bold text-xs text-[var(--rz-text-secondary)] shrink-0">
            {name.charAt(0).toUpperCase()}
          </div>
        )}

        <div className="min-w-0">
          <div className="flex items-center gap-1.5 leading-tight">
            <span className="font-bold text-xs sm:text-[13px] text-[var(--rz-text)] truncate">
              {name}
            </span>
            {verified && (
              <span title="Verified Creator Keyring" className="inline-flex">
                <ShieldCheck className="w-3.5 h-3.5 text-sky-400 shrink-0" />
              </span>
            )}
          </div>
          <div className="text-[11px] text-[var(--rz-text-secondary)] mt-0.5 flex items-center gap-2">
            <span>Author</span>
            {(packageCount || totalDownloads) && (
              <>
                <span className="text-[var(--rz-border-strong)]">·</span>
                <span>
                  {packageCount ? `${packageCount} packages` : ""}
                  {packageCount && totalDownloads ? " · " : ""}
                  {totalDownloads ? `${(totalDownloads / 1000).toFixed(0)}k downloads` : ""}
                </span>
              </>
            )}
          </div>
        </div>
      </div>

      <button
        type="button"
        onClick={() => setFollowing((f) => !f)}
        className={[
          "px-2.5 py-1 rounded-lg text-xs font-semibold border transition-all cursor-pointer select-none shrink-0 flex items-center gap-1",
          following
            ? "bg-[var(--rz-surface-elevated)] border-[var(--rz-accent)] text-[var(--rz-accent-text)]"
            : "bg-[var(--rz-surface-elevated)] border-[var(--rz-border-subtle)] text-[var(--rz-text-secondary)] hover:text-[var(--rz-text)] hover:border-[var(--rz-border-strong)]",
        ].join(" ")}
      >
        {following ? (
          <>
            <UserCheck className="w-3 h-3 text-emerald-400" />
            <span>Following</span>
          </>
        ) : (
          <>
            <UserPlus className="w-3 h-3 text-[var(--rz-text-muted)]" />
            <span>Follow</span>
          </>
        )}
      </button>
    </div>
  );
};
