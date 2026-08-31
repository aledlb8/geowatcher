const PATHS = {
  signal:
    "M5.5 8.5a9 9 0 0 0 0 7M18.5 8.5a9 9 0 0 1 0 7M8.8 10.6a4 4 0 0 0 0 2.8M15.2 10.6a4 4 0 0 1 0 2.8",
  sliders: "M4 7h5M14 7h6M4 17h10M19 17h1M4 12h2M11 12h9",
  plus: "M12 5.5v13M5.5 12h13",
  pause: "M9.5 5.5v13M14.5 5.5v13",
  play: "M8.5 5.6v12.8L19 12z",
  trash:
    "M4.5 7h15M9.5 7V5.2a1 1 0 0 1 1-1h3a1 1 0 0 1 1 1V7M6.5 7l.9 12a1 1 0 0 0 1 .9h7.2a1 1 0 0 0 1-.9L17.5 7",
  folder: "M3.5 7a1 1 0 0 1 1-1H9l2 2h9a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1H4.5a1 1 0 0 1-1-1z",
  alert:
    "M12 9.2v4M12 16.6h.01M10.6 4.4 2.9 17.6a1.6 1.6 0 0 0 1.4 2.4h15.4a1.6 1.6 0 0 0 1.4-2.4L13.4 4.4a1.6 1.6 0 0 0-2.8 0z",
  check: "M5 12.5 9.5 17 19 6.5",
} as const;

export type IconName = keyof typeof PATHS;

export function Icon({ name, size = 16 }: { name: IconName; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.6}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={PATHS[name]} />
      {name === "signal" && <circle cx="12" cy="12" r="1.6" fill="currentColor" stroke="none" />}
      {name === "sliders" && (
        <>
          <circle cx="11.5" cy="7" r="2.1" />
          <circle cx="8.5" cy="12" r="2.1" />
          <circle cx="16.5" cy="17" r="2.1" />
        </>
      )}
    </svg>
  );
}
