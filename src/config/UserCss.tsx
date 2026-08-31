import { useEffect, useState } from "react";
import { getUserCss, setUserCss } from "../lib/api";
import { Button, Card, INPUT } from "../ui/primitives";

const TARGETS = [".gw-card", ".gw-row", ".gw-value", ".gw-bar", ".gw-tick"];

/** The escape hatch for anything the controls above do not cover. */
export function UserCss() {
  const [css, setCss] = useState("");
  const [saved, setSaved] = useState(true);

  useEffect(() => {
    void getUserCss().then((text) => {
      setCss(text);
      setSaved(true);
    });
  }, []);

  return (
    <Card
      title="Custom CSS"
      note={saved ? "applied" : "unsaved"}
      footer={
        <>
          <Button
            variant="primary"
            icon="check"
            disabled={saved}
            onClick={() => void setUserCss(css).then(() => setSaved(true))}
          >
            {saved ? "Applied" : "Apply CSS"}
          </Button>
          <span className="font-mono text-[10px] text-overlay">
            widget.css, next to settings.json
          </span>
        </>
      }
    >
      <p className="mb-3 text-[12.5px] leading-relaxed text-overlay">
        Layered on top of everything above. Target{" "}
        {TARGETS.map((name, i) => (
          <span key={name}>
            {i > 0 && ", "}
            <code className="font-mono text-[11px] text-muted">{name}</code>
          </span>
        ))}
        .
      </p>
      <textarea
        value={css}
        spellCheck={false}
        rows={8}
        aria-label="Custom CSS"
        placeholder=".gw-card { border-width: 2px; }"
        onChange={(e) => {
          setCss(e.currentTarget.value);
          setSaved(false);
        }}
        className={`${INPUT} resize-y py-3 font-mono text-[11.5px] leading-relaxed text-muted`}
      />
    </Card>
  );
}
