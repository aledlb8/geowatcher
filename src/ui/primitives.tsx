import type { CSSProperties, ReactNode } from "react";
import { Icon, type IconName } from "./Icon";

export const INPUT =
  "w-full min-w-0 rounded-md border border-surface0 bg-mantle px-3 py-2 text-[13px] text-fg outline-none placeholder:text-overlay focus:border-blue";

export const LABEL = "text-[11px] font-medium text-muted";

/** Column headers and card meta only — never body labels. */
export const HEADING = "font-mono text-[10px] uppercase tracking-[0.14em] text-overlay";

export function Card({
  title,
  note,
  actions,
  flush,
  footer,
  children,
}: {
  title: string;
  note?: ReactNode;
  actions?: ReactNode;
  flush?: boolean;
  footer?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="rounded-lg border border-surface0 bg-base">
      <header className="flex items-center gap-3 border-b border-surface0 px-5 py-3">
        <h2 className="text-[13px] font-semibold text-fg">{title}</h2>
        {note !== undefined && <span className={`${HEADING} ml-auto`}>{note}</span>}
        {actions && <span className={note === undefined ? "ml-auto" : ""}>{actions}</span>}
      </header>
      <div className={flush ? "" : "p-5"}>{children}</div>
      {footer && (
        <footer className="flex items-center gap-3 border-t border-surface0 bg-mantle px-5 py-3">
          {footer}
        </footer>
      )}
    </section>
  );
}

export function Field({
  label,
  value,
  children,
}: {
  label: string;
  value?: string;
  children: ReactNode;
}) {
  return (
    <div>
      <div className="mb-2 flex items-baseline justify-between gap-3">
        <span className={LABEL}>{label}</span>
        {value !== undefined && (
          <span className="font-mono text-[10px] tabular-nums text-overlay">{value}</span>
        )}
      </div>
      {children}
    </div>
  );
}

export function Slider({
  min,
  max,
  step = 1,
  value,
  label,
  onChange,
  onCommit,
}: {
  min: number;
  max: number;
  step?: number;
  value: number;
  label?: string;
  onChange: (value: number) => void;
  /** Fired when the drag ends, for changes too expensive to run per frame. */
  onCommit?: (value: number) => void;
}) {
  const settle =
    onCommit &&
    ((e: { currentTarget: HTMLInputElement }) => onCommit(Number(e.currentTarget.value)));

  return (
    <input
      type="range"
      className="range"
      min={min}
      max={max}
      step={step}
      value={value}
      aria-label={label}
      onChange={(e) => onChange(Number(e.currentTarget.value))}
      onPointerUp={settle}
      onKeyUp={settle}
      onBlur={settle}
      style={{ "--fill": `${((value - min) / (max - min)) * 100}%` } as CSSProperties}
    />
  );
}

export function Switch({
  label,
  hint,
  checked,
  onChange,
}: {
  label: string;
  hint?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className="flex cursor-pointer items-start gap-3">
      <span
        className={`relative mt-px block h-4 w-7 flex-none rounded-full border transition-colors ${
          checked ? "border-blue bg-blue" : "border-surface1 bg-surface0"
        }`}
      >
        <input
          type="checkbox"
          checked={checked}
          onChange={(e) => onChange(e.currentTarget.checked)}
          className="absolute inset-0 h-full w-full cursor-pointer opacity-0"
        />
        <span
          className={`absolute top-[2px] block h-[10px] w-[10px] rounded-full transition-all ${
            checked ? "left-[14px] bg-crust" : "left-[2px] bg-surface2"
          }`}
        />
      </span>
      <span className="min-w-0">
        <span className="block text-[13px] text-fg">{label}</span>
        {hint && (
          <span className="mt-0.5 block text-[11.5px] leading-snug text-overlay">{hint}</span>
        )}
      </span>
    </label>
  );
}

export function Swatch({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <span className="flex items-center gap-2">
      <span
        className="relative h-8 w-8 flex-none overflow-hidden rounded-md border border-surface1"
        style={{ background: value }}
      >
        <input
          type="color"
          value={value}
          aria-label={label}
          onChange={(e) => onChange(e.currentTarget.value)}
          className="absolute -inset-2 h-[calc(100%+1rem)] w-[calc(100%+1rem)] cursor-pointer border-0 bg-transparent p-0"
        />
      </span>
      <input
        type="text"
        value={value}
        spellCheck={false}
        aria-label={`${label} hex`}
        onChange={(e) => onChange(e.currentTarget.value)}
        className={`${INPUT} px-2 py-1.5 font-mono text-[11px] uppercase`}
      />
    </span>
  );
}

export function Segmented<T extends string>({
  options,
  value,
  onChange,
}: {
  options: Array<{ value: T; label: string }>;
  value: T;
  onChange: (value: T) => void;
}) {
  return (
    <span className="inline-flex rounded-md border border-surface0 bg-mantle p-0.5">
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          aria-pressed={option.value === value}
          onClick={() => onChange(option.value)}
          className={`rounded px-2.5 py-1 text-[11px] transition-colors ${
            option.value === value ? "bg-surface0 text-fg" : "text-overlay hover:text-muted"
          }`}
        >
          {option.label}
        </button>
      ))}
    </span>
  );
}

export function Button({
  variant = "default",
  icon,
  disabled,
  type = "button",
  onClick,
  children,
}: {
  variant?: "default" | "primary" | "ghost";
  icon?: IconName;
  disabled?: boolean;
  type?: "button" | "submit";
  onClick?: () => void;
  children: ReactNode;
}) {
  const skin = {
    default: "border border-surface1 bg-surface0/50 text-fg hover:bg-surface0",
    primary: "bg-blue font-semibold text-crust hover:bg-lavender",
    ghost: "text-muted hover:bg-surface0/60 hover:text-fg",
  }[variant];

  return (
    <button
      type={type === "submit" ? "submit" : "button"}
      disabled={disabled}
      onClick={onClick}
      className={`inline-flex items-center justify-center gap-2 rounded-md px-3.5 py-2 text-[12.5px] whitespace-nowrap transition-colors disabled:pointer-events-none disabled:opacity-40 ${skin}`}
    >
      {icon && <Icon name={icon} size={14} />}
      {children}
    </button>
  );
}

export function IconButton({
  icon,
  label,
  danger,
  onClick,
}: {
  icon: IconName;
  label: string;
  danger?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      onClick={onClick}
      className={`grid h-7 w-7 place-items-center rounded-md text-overlay transition-colors hover:bg-surface0 ${
        danger ? "hover:text-red" : "hover:text-fg"
      }`}
    >
      <Icon name={icon} size={14} />
    </button>
  );
}
