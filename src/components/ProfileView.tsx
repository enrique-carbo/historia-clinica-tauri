// src/components/ProfileView.tsx

import { useState, useEffect } from "react";
import { useAuthStore } from "../stores/useAuthStore";
import { invoke } from "@tauri-apps/api/core";

type License = { number: string; jurisdiction: string };

type ProfileState = {
  full_name: string;
  profession: string;
  specialty: string;
  licenses: License[];
  city: string;
  country: string;
  address: string;
  phone: string;
  email: string;
  website: string;
};

const EMPTY_PROFILE: ProfileState = {
  full_name: "",
  profession: "",
  specialty: "",
  licenses: [],
  city: "",
  country: "",
  address: "",
  phone: "",
  email: "",
  website: "",
};

const PROFESSION_SUGGESTIONS = [
  "Médico/a",
  "Enfermero/a",
  "Lic. en Psicología",
  "Kinesiólogo/a",
  "Odontólogo/a",
  "Farmacéutico/a",
  "Nutricionista",
  "Fonoaudiólogo/a",
  "Lic. en Trabajo Social",
  "Técnico/a en Enfermería",
];

export function ProfileView() {
  const { activeUser } = useAuthStore();
  const [profile, setProfile] = useState<ProfileState>(EMPTY_PROFILE);
  const [isEditing, setIsEditing] = useState(false);
  const [loading, setLoading] = useState(false);

  const [pwdForm, setPwdForm] = useState({
    current: "",
    new: "",
    confirm: "",
  });
  const [pwdLoading, setPwdLoading] = useState(false);
  const [pwdMsg, setPwdMsg] = useState<{ type: "ok" | "err"; text: string } | null>(
    null
  );

  useEffect(() => {
    if (activeUser?.user_id) loadProfile();
  }, [activeUser]);

  const loadProfile = async () => {
    try {
      const data = await invoke<any>("get_my_profile");
      setProfile({
        ...EMPTY_PROFILE,
        ...data,
        licenses: Array.isArray(data.licenses) ? data.licenses : [],
      });
    } catch (e) {
      console.error("Error cargando perfil:", e);
    }
  };

  const handleSave = async () => {
    if (!activeUser) return;
    setLoading(true);
    try {
      await invoke("update_my_profile", {
        input: {
          user_id: activeUser.user_id,
          ...profile,
        },
      });
      setIsEditing(false);
      await loadProfile();
    } catch (e) {
      console.error("Error guardando perfil:", e);
    } finally {
      setLoading(false);
    }
  };

  const handleChange = (field: string, value: string) => {
    setProfile((p) => ({ ...p, [field]: value }));
  };

  const handleLicenseChange = (index: number, field: keyof License, value: string) => {
    setProfile((p) => ({
      ...p,
      licenses: p.licenses.map((l, i) => (i === index ? { ...l, [field]: value } : l)),
    }));
  };

  const addLicense = () => {
    setProfile((p) => ({
      ...p,
      licenses: [...p.licenses, { number: "", jurisdiction: "" }],
    }));
  };

  const removeLicense = (index: number) => {
    setProfile((p) => ({
      ...p,
      licenses: p.licenses.filter((_, i) => i !== index),
    }));
  };

  const handleChangePassword = async () => {
    if (!activeUser) return;
    setPwdMsg(null);

    if (pwdForm.new.length < 8) {
      setPwdMsg({
        type: "err",
        text: "La nueva contraseña debe tener al menos 8 caracteres.",
      });
      return;
    }
    if (pwdForm.new !== pwdForm.confirm) {
      setPwdMsg({ type: "err", text: "Las contraseñas nuevas no coinciden." });
      return;
    }

    setPwdLoading(true);
    try {
      await invoke("change_password", {
        userId: activeUser.user_id,
        currentPassword: pwdForm.current,
        newPassword: pwdForm.new,
      });
      setPwdMsg({ type: "ok", text: "Contraseña cambiada correctamente." });
      setPwdForm({ current: "", new: "", confirm: "" });
    } catch (e) {
      setPwdMsg({ type: "err", text: String(e) });
    } finally {
      setPwdLoading(false);
    }
  };

  return (
    <div className="max-w-4xl mx-auto p-8 bg-zinc-900 rounded-xl border border-zinc-800 shadow-lg">
      <div className="flex justify-between items-center mb-8 border-b border-zinc-800 pb-4">
        <h2 className="text-2xl font-bold text-zinc-100 flex items-center gap-2">
          👤 <span className="hidden md:inline">Datos del profesional</span>
        </h2>
        {!isEditing && (
          <button
            onClick={() => setIsEditing(true)}
            className="rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 transition-colors"
          >
            Editar
          </button>
        )}
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-8">
        {/* Columna Izquierda: Datos Clínicos */}
        <div className="space-y-5">
          <h3 className="text-xs font-bold text-blue-400 uppercase tracking-widest mb-4">
            Identidad Clínica
          </h3>

          {(
            [
              { label: "Nombre Completo", field: "full_name" },
              { label: "Profesión", field: "profession", list: "profession-suggestions" },
              { label: "Especialidad", field: "specialty" },
            ] as { label: string; field: string; list?: string }[]
          ).map((item) => (
            <div key={item.field}>
              <label className="block text-xs font-semibold text-zinc-500 mb-1.5 uppercase">
                {item.label}
              </label>
              {isEditing ? (
                <>
                  <input
                    list={item.list}
                    value={(profile as any)[item.field]}
                    onChange={(e) => handleChange(item.field, e.target.value)}
                    className="w-full rounded-lg bg-zinc-950 border border-zinc-700 px-3 py-2.5 text-sm text-zinc-200 focus:border-blue-500 outline-none transition-colors"
                  />
                  {item.list && (
                    <datalist id={item.list}>
                      {PROFESSION_SUGGESTIONS.map((s) => (
                        <option key={s} value={s} />
                      ))}
                    </datalist>
                  )}
                </>
              ) : (
                <p className="text-sm text-zinc-300 font-medium py-2.5 border-b border-zinc-800/50">
                  {(profile as any)[item.field] || "—"}
                </p>
              )}
            </div>
          ))}

          {/* Matrículas profesionales (múltiples: número + jurisdicción) */}
          <div>
            <label className="block text-xs font-semibold text-zinc-500 mb-1.5 uppercase">
              Matrículas Profesionales
            </label>
            {isEditing ? (
              <div className="space-y-2">
                {profile.licenses.map((lic, i) => (
                  <div key={i} className="flex gap-2">
                    <input
                      value={lic.number}
                      placeholder="Número (ej. MP-12345)"
                      onChange={(e) => handleLicenseChange(i, "number", e.target.value)}
                      className="flex-1 min-w-0 rounded-lg bg-zinc-950 border border-zinc-700 px-3 py-2.5 text-sm text-zinc-200 focus:border-blue-500 outline-none transition-colors"
                    />
                    <input
                      value={lic.jurisdiction}
                      placeholder="Jurisdicción (ej. Córdoba)"
                      onChange={(e) =>
                        handleLicenseChange(i, "jurisdiction", e.target.value)
                      }
                      className="flex-1 min-w-0 rounded-lg bg-zinc-950 border border-zinc-700 px-3 py-2.5 text-sm text-zinc-200 focus:border-blue-500 outline-none transition-colors"
                    />
                    <button
                      type="button"
                      onClick={() => removeLicense(i)}
                      title="Quitar matrícula"
                      className="rounded-lg border border-zinc-700 px-3 text-sm text-zinc-400 hover:border-red-700 hover:text-red-400 transition-colors"
                    >
                      ✕
                    </button>
                  </div>
                ))}
                <div className="flex items-center gap-3 pt-1">
                  <button
                    type="button"
                    onClick={addLicense}
                    className="rounded-lg border border-dashed border-zinc-700 px-3 py-2 text-xs font-semibold text-zinc-400 hover:border-blue-600 hover:text-blue-400 transition-colors"
                  >
                    + Agregar matrícula
                  </button>
                  <span className="text-xs text-zinc-600">
                    La primera es la matrícula principal.
                  </span>
                </div>
              </div>
            ) : profile.licenses.length > 0 ? (
              <ul className="space-y-1.5 py-2 border-b border-zinc-800/50">
                {profile.licenses.map((lic, i) => (
                  <li key={i} className="text-sm text-zinc-300 font-medium">
                    {lic.number.trim() || "—"}
                    {lic.jurisdiction.trim() && (
                      <span className="text-zinc-500"> — {lic.jurisdiction.trim()}</span>
                    )}
                    {i === 0 && (
                      <span className="ml-2 text-[10px] uppercase tracking-wider text-blue-500">
                        principal
                      </span>
                    )}
                  </li>
                ))}
              </ul>
            ) : (
              <p className="text-sm text-zinc-300 font-medium py-2.5 border-b border-zinc-800/50">
                —
              </p>
            )}
          </div>
        </div>

        {/* Columna Derecha: Contacto Profesional */}
        <div className="space-y-5">
          <h3 className="text-xs font-bold text-emerald-400 uppercase tracking-widest mb-4">
            Datos de Contacto
          </h3>

          {[
            { label: "Domicilio Profesional", field: "address", type: "text" },
            { label: "Ciudad", field: "city", type: "text" },
            { label: "País", field: "country", type: "text" },
            { label: "Teléfono de Consultorio", field: "phone", type: "tel" },
            { label: "Correo Electrónico", field: "email", type: "email" },
            { label: "Sitio Web / Agenda", field: "website", type: "url" },
          ].map((item) => (
            <div key={item.field}>
              <label className="block text-xs font-semibold text-zinc-500 mb-1.5 uppercase">
                {item.label}
              </label>
              {isEditing ? (
                <input
                  type={item.type}
                  value={(profile as any)[item.field]}
                  onChange={(e) => handleChange(item.field, e.target.value)}
                  className="w-full rounded-lg bg-zinc-950 border border-zinc-700 px-3 py-2.5 text-sm text-zinc-200 focus:border-emerald-500 outline-none transition-colors"
                />
              ) : (
                <p className="text-sm text-zinc-300 font-medium py-2.5 border-b border-zinc-800/50 truncate">
                  {(profile as any)[item.field] || "—"}
                </p>
              )}
            </div>
          ))}
        </div>
      </div>

      {/* Sección: Seguridad / Cambiar contraseña */}
      <div className="mt-8 pt-6 border-t border-zinc-800">
        <h3 className="text-xs font-bold text-amber-400 uppercase tracking-widest mb-4">
          🔒 Seguridad — Cambiar contraseña
        </h3>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-5 max-w-3xl">
          <div>
            <label className="block text-xs font-semibold text-zinc-500 mb-1.5 uppercase">
              Contraseña Actual
            </label>
            <input
              type="password"
              autoComplete="current-password"
              value={pwdForm.current}
              onChange={(e) => setPwdForm((p) => ({ ...p, current: e.target.value }))}
              className="w-full rounded-lg bg-zinc-950 border border-zinc-700 px-3 py-2.5 text-sm text-zinc-200 focus:border-amber-500 outline-none transition-colors"
            />
          </div>
          <div>
            <label className="block text-xs font-semibold text-zinc-500 mb-1.5 uppercase">
              Nueva Contraseña
            </label>
            <input
              type="password"
              autoComplete="new-password"
              value={pwdForm.new}
              onChange={(e) => setPwdForm((p) => ({ ...p, new: e.target.value }))}
              className="w-full rounded-lg bg-zinc-950 border border-zinc-700 px-3 py-2.5 text-sm text-zinc-200 focus:border-amber-500 outline-none transition-colors"
            />
          </div>
          <div>
            <label className="block text-xs font-semibold text-zinc-500 mb-1.5 uppercase">
              Repetir Nueva
            </label>
            <input
              type="password"
              autoComplete="new-password"
              value={pwdForm.confirm}
              onChange={(e) => setPwdForm((p) => ({ ...p, confirm: e.target.value }))}
              className="w-full rounded-lg bg-zinc-950 border border-zinc-700 px-3 py-2.5 text-sm text-zinc-200 focus:border-amber-500 outline-none transition-colors"
            />
          </div>
        </div>

        <div className="mt-4 flex items-center gap-4">
          <button
            onClick={handleChangePassword}
            disabled={
              pwdLoading || !pwdForm.current || !pwdForm.new || !pwdForm.confirm
            }
            className="rounded-lg bg-amber-600 px-5 py-2.5 text-sm font-bold text-white hover:bg-amber-700 disabled:opacity-50 transition-colors"
          >
            {pwdLoading ? "Cambiando..." : "Cambiar Contraseña"}
          </button>

          {pwdMsg && (
            <span
              className={`text-sm font-medium ${
                pwdMsg.type === "ok" ? "text-emerald-400" : "text-red-400"
              }`}
            >
              {pwdMsg.type === "ok" ? "✓ " : "✕ "}
              {pwdMsg.text}
            </span>
          )}
        </div>

        <p className="mt-3 text-xs text-zinc-500">
          Cambiar la contraseña re-cifra la bóveda local y el wrap de datos. Tu
          frase semilla en papel no se ve afectada.
        </p>
      </div>

      {isEditing && (
        <div className="mt-8 pt-6 border-t border-zinc-800 flex justify-end gap-3">
          <button
            onClick={() => setIsEditing(false)}
            className="rounded-lg bg-zinc-800 px-6 py-2.5 text-sm font-bold text-zinc-300 hover:bg-zinc-700 transition-colors"
          >
            Cancelar
          </button>
          <button
            onClick={handleSave}
            disabled={loading}
            className="rounded-lg bg-emerald-600 px-6 py-2.5 text-sm font-bold text-white hover:bg-emerald-700 disabled:opacity-50 shadow-lg shadow-emerald-900/20 transition-colors"
          >
            {loading ? "Guardando cambios..." : "Guardar Perfil"}
          </button>
        </div>
      )}
    </div>
  );
}
