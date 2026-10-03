import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../ui/Button";
import { Alert } from "../ui/Alert";
import { Input } from "../ui/Input";

type Phase = "idle" | "shown" | "done";

export function AdminSeedPanel() {
  const [phase, setPhase] = useState<Phase>("idle");
  const [phrase, setPhrase] = useState("");
  const [confirmInput, setConfirmInput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleGenerate = async () => {
    setError(null);
    setLoading(true);
    try {
      const generated = await invoke<string>("generate_seed");
      setPhrase(generated);
      setConfirmInput("");
      setPhase("shown");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleConfirmRotate = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    setLoading(true);
    try {
      const isValid = await invoke<boolean>("verify_seed", {
        phrase: confirmInput.trim(),
      });
      if (!isValid) {
        setError("La frase no coincide con la generada. Revisá el orden de las palabras.");
        return;
      }
      await invoke("admin_rotate_seed", { phrase: confirmInput.trim() });
      setPhase("done");
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleAbort = () => {
    setPhase("idle");
    setPhrase("");
    setConfirmInput("");
    setError(null);
  };

  return (
    <div className="max-w-3xl mx-auto flex flex-col gap-5">
      <div>
        <h2 className="text-lg font-semibold text-zinc-300">Frase Semilla</h2>
        <p className="text-xs text-zinc-500 mt-1">
          La frase de 6 palabras es el respaldo maestro de esta instalación: sirve
          para recuperar los datos cifrados si un usuario pierde su contraseña.
        </p>
      </div>

      <div className="bg-red-950/30 border border-red-800/50 rounded-lg p-4">
        <p className="text-xs text-red-400">
          <strong className="text-red-300">⚠️ Rotar la frase invalida la anterior.</strong>{" "}
          Si no anotás la nueva en papel, la frase vieja deja de servir y no habrá
          forma de recuperar los datos si se pierden todas las contraseñas. La
          rotación recién se aplica cuando confirmás re-escribiéndola.
        </p>
      </div>

      {error && (
        <Alert variant="error" className="my-0">
          <div className="flex justify-between gap-4">
            <span>{error}</span>
            <button onClick={() => setError(null)} className="font-bold" aria-label="Cerrar">
              ✕
            </button>
          </div>
        </Alert>
      )}

      {phase === "idle" && (
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
          <p className="text-sm text-zinc-400 mb-4">
            Al rotar, se generará una frase nueva de 6 palabras que reemplazará a la
            actual.
          </p>
          <Button variant="danger" onClick={handleGenerate} isLoading={loading}>
            Generar frase nueva
          </Button>
        </div>
      )}

      {phase === "shown" && (
        <form onSubmit={handleConfirmRotate} className="bg-zinc-900 border border-zinc-800 rounded-xl p-6 flex flex-col gap-5">
          <div>
            <p className="text-xs font-bold text-amber-400 uppercase tracking-wider mb-3">
              1. Anotá esta frase en papel — no está aplicada todavía
            </p>
            <div className="bg-zinc-950 border border-amber-800/50 rounded-lg p-4 grid grid-cols-2 md:grid-cols-3 gap-2">
              {phrase.split(" ").map((word, i) => (
                <span key={i} className="text-sm font-mono text-zinc-200">
                  <span className="text-zinc-600 mr-2">{i + 1}.</span>
                  {word}
                </span>
              ))}
            </div>
          </div>

          <div>
            <p className="text-xs font-bold text-amber-400 uppercase tracking-wider mb-2">
              2. Re-escribí la frase para confirmar que la anotaste
            </p>
            <Input
              type="text"
              label="Frase semilla completa"
              value={confirmInput}
              onChange={(e) => setConfirmInput(e.target.value)}
              required
              placeholder="palabra1 palabra2 ... palabra6"
            />
          </div>

          <div className="flex justify-end gap-3">
            <Button variant="ghost" type="button" onClick={handleAbort}>
              Cancelar (no rotar)
            </Button>
            <Button variant="danger" type="submit" isLoading={loading}>
              Confirmar rotación
            </Button>
          </div>
        </form>
      )}

      {phase === "done" && (
        <div className="bg-zinc-900 border border-emerald-800/50 rounded-xl p-6 flex flex-col gap-4">
          <Alert variant="success" className="my-0">
            Frase semilla rotada. La frase anterior quedó invalidada.
          </Alert>
          <p className="text-sm text-zinc-400">
            Guardá la nueva frase en papel en un lugar seguro. Es la única forma de
            recuperar los datos si se pierden todas las contraseñas.
          </p>
          <div className="flex justify-end">
            <Button variant="secondary" onClick={handleAbort}>
              Entendido
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
