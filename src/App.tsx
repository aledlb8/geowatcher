import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import reactLogo from "./assets/react.svg";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");

  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    setGreetMsg(await invoke("greet", { name }));
  }

  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-8 bg-zinc-50 p-8 text-zinc-900 dark:bg-zinc-950 dark:text-zinc-100">
      <h1 className="text-3xl font-semibold tracking-tight">Welcome to Tauri + React</h1>

      <div className="flex items-center gap-8">
        <a href="https://vite.dev" target="_blank" rel="noreferrer">
          <img
            src="/vite.svg"
            className="h-24 w-24 p-4 transition-[filter] duration-700 hover:drop-shadow-[0_0_2em_#747bff]"
            alt="Vite logo"
          />
        </a>
        <a href="https://tauri.app" target="_blank" rel="noreferrer">
          <img
            src="/tauri.svg"
            className="h-24 w-24 p-4 transition-[filter] duration-700 hover:drop-shadow-[0_0_2em_#24c8db]"
            alt="Tauri logo"
          />
        </a>
        <a href="https://react.dev" target="_blank" rel="noreferrer">
          <img
            src={reactLogo}
            className="h-24 w-24 p-4 transition-[filter] duration-700 hover:drop-shadow-[0_0_2em_#61dafb]"
            alt="React logo"
          />
        </a>
      </div>

      <p className="text-sm text-zinc-500 dark:text-zinc-400">
        Click on the Tauri, Vite, and React logos to learn more.
      </p>

      <form
        className="flex items-center gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          greet();
        }}
      >
        <input
          id="greet-input"
          value={name}
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="Enter a name..."
          className="rounded-lg border border-zinc-300 bg-white px-4 py-2 outline-none focus:border-transparent focus:ring-2 focus:ring-sky-500 dark:border-zinc-700 dark:bg-zinc-900"
        />
        <button
          type="submit"
          className="rounded-lg bg-sky-600 px-4 py-2 font-medium text-white transition-colors hover:bg-sky-500 active:bg-sky-700"
        >
          Greet
        </button>
      </form>

      <p className="min-h-6 text-lg">{greetMsg}</p>
    </main>
  );
}

export default App;
