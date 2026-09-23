import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface SeedPhraseSetupProps {
  onComplete: (seedVerified: boolean) => void;
}

type Step = "generate" | "display" | "verify" | "success";

export function SeedPhraseSetup({ onComplete }: SeedPhraseSetupProps) {
  const [step, setStep] = useState<Step>("generate");
  const [phrase, setPhrase] = useState("");
  const [verifyInput, setVerifyInput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const handleGenerate = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const generated = await invoke<string>("generate_seed");
      setPhrase(generated);
      setStep("display");
    } catch (err) {
      setError(String(err));
    } finally {
      setIsLoading(false);
    }
  };

  const handleVerify = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const isValid = await invoke<boolean>("verify_seed", { phrase: verifyInput });
      if (isValid) {
        // Persistir master wrap (.data_key.master) con la seed-derived key
        await invoke("setup_seed_master_wrap", { phrase: verifyInput.trim() });
        setStep("success");
        setTimeout(() => onComplete(true), 1500);
      } else {
        setError("La frase no coincide. Verificá las palabras.");
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setIsLoading(false);
    }
  };

  const handleSkip = () => {
    // No marca como configurada - permitir configurar después
    setStep("generate");
  };

  return (
    <div className="min-h-screen bg-zinc-950 flex items-center justify-center p-4">
      <div className="w-full max-w-md">
        {/* Paso 1: Generar */}
        {step === "generate" && (
          <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6 text-center">
            <div className="text-4xl mb-4">🔐</div>
            <h2 className="text-xl font-bold text-zinc-100 mb-2">Frase Semilla</h2>
            <p className="text-sm text-zinc-400 mb-6">
              Generá una frase de 6 palabras que servirá como capa adicional de seguridad.
              <br />
              <strong className="text-zinc-300">Guardala en un lugar seguro (placa de metal).</strong>
            </p>
            <button
              onClick={handleGenerate}
              disabled={isLoading}
              className="w-full px-4 py-3 bg-blue-600 hover:bg-blue-500 disabled:bg-zinc-700 text-white rounded-lg font-medium transition-colors"
            >
              {isLoading ? "Generando..." : "Generar Frase Semilla"}
            </button>
            <button
              onClick={handleSkip}
              className="w-full mt-3 px-4 py-2 text-zinc-500 hover:text-zinc-300 text-sm transition-colors"
            >
              Omitir por ahora
            </button>
          </div>
        )}

        {/* Paso 2: Mostrar frase */}
        {step === "display" && (
          <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
            <div className="text-center mb-4">
              <div className="text-4xl mb-2">📝</div>
              <h2 className="text-xl font-bold text-zinc-100">Tu Frase Semilla</h2>
              <p className="text-xs text-zinc-500 mt-1">
                Anotala y guardala en un lugar seguro
              </p>
            </div>

            {/* Palabras */}
            <div className="grid grid-cols-2 gap-2 mb-6">
              {phrase.split(" ").map((word, i) => (
                <div
                  key={i}
                  className="flex items-center gap-2 px-3 py-2 bg-zinc-950 border border-zinc-700 rounded-lg"
                >
                  <span className="text-xs text-zinc-500 w-4">{i + 1}.</span>
                  <span className="text-sm font-mono text-zinc-200">{word}</span>
                </div>
              ))}
            </div>

            {/* Advertencia */}
            <div className="p-3 bg-yellow-950/30 border border-yellow-800/50 rounded-lg mb-4">
              <p className="text-xs text-yellow-400">
                ⚠️ <strong>Importante:</strong> Si perdés esta frase y tu computadora se daña, los datos son irrecuperables.
              </p>
            </div>

            <button
              onClick={() => setStep("verify")}
              className="w-full px-4 py-3 bg-blue-600 hover:bg-blue-500 text-white rounded-lg font-medium transition-colors"
            >
              Continuar
            </button>
          </div>
        )}

        {/* Paso 3: Verificar */}
        {step === "verify" && (
          <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6">
            <div className="text-center mb-4">
              <div className="text-4xl mb-2">✅</div>
              <h2 className="text-xl font-bold text-zinc-100">Verificar Frase</h2>
              <p className="text-sm text-zinc-400">
                Ingresá las 6 palabras para confirmar que las guardaste
              </p>
            </div>

            <input
              type="text"
              value={verifyInput}
              onChange={(e) => setVerifyInput(e.target.value)}
              placeholder="Escribí las 6 palabras separadas por espacio"
              className="w-full px-4 py-3 bg-zinc-950 border border-zinc-700 rounded-lg text-zinc-200 text-sm placeholder-zinc-600 focus:outline-none focus:border-blue-600 mb-4"
            />

            {error && (
              <div className="p-3 bg-red-950/30 border border-red-800/50 rounded-lg mb-4">
                <p className="text-xs text-red-400">{error}</p>
              </div>
            )}

            <button
              onClick={handleVerify}
              disabled={!verifyInput.trim() || isLoading}
              className="w-full px-4 py-3 bg-blue-600 hover:bg-blue-500 disabled:bg-zinc-700 text-white rounded-lg font-medium transition-colors"
            >
              {isLoading ? "Verificando..." : "Verificar"}
            </button>
          </div>
        )}

        {/* Paso 4: Éxito */}
        {step === "success" && (
          <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6 text-center">
            <div className="text-4xl mb-4">🎉</div>
            <h2 className="text-xl font-bold text-zinc-100 mb-2">¡Frase Verificada!</h2>
            <p className="text-sm text-zinc-400">
              Tu frase semilla está configurada correctamente.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
