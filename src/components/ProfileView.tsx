// src/components/ProfileView.tsx

import { useState, useEffect } from "react";
import { useAuthStore } from "../stores/useAuthStore";
import { invoke } from "@tauri-apps/api/core";
import { ChangePasswordForm } from "./ChangePasswordForm";
import { Alert } from "./ui/Alert";

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
];

export function ProfileView() {
  const { activeUser } = useAuthStore();
  const [profile, setProfile] = useState<ProfileState>(EMPTY_PROFILE);
  const [isEditing, setIsEditing] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

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
      setError(null);
    } catch (e) {
      setError(`No se pudo cargar el perfil: ${e}`);
    }
  };

  const handleSave = async () => {
    if (!activeUser) return;
    setLoading(true);
    setError(null);
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
      setError(`No se pudo guardar el perfil: ${e}`);
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

  return (
    <div className="max-w-4xl mx-auto p-8 bg-surface rounded-xl border border-border shadow-lg">
      {error && (
        <Alert variant="error" className="mb-6">
          <div className="flex justify-between gap-4">
            <span>{error}</span>
            <button
              onClick={() => setError(null)}
              className="font-bold"
              aria-label="Cerrar"
            >
              ✕
            </button>
          </div>
        </Alert>
      )}
      <div className="flex justify-between items-center mb-8 border-b border-border pb-4">
        <h2 className="text-2xl font-bold text-text-primary flex items-center gap-2">
          <span aria-hidden="true">👤</span> <span className="hidden md:inline">Datos del profesional</span>
        </h2>
        {!isEditing && (
          <button
            onClick={() => setIsEditing(true)}
            className="rounded-lg bg-action-primary px-4 py-2 text-sm font-medium text-text-inverse hover:bg-action-primary-hover transition-colors"
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
              { label: "Profesión", field: "profession" },
              { label: "Especialidad", field: "specialty" },
            ] as { label: string; field: string }[]
          ).map((item) => (
            <div key={item.field}>
              <label
                htmlFor={`profile-${item.field}`}
                className="block text-xs font-semibold text-text-muted mb-1.5 uppercase"
              >
                {item.label}
              </label>
              {isEditing ? (
                item.field === "profession" ? (
                  <div className="space-y-2">
                    <select
                      id="profile-profession"
                      value={
                        PROFESSION_SUGGESTIONS.includes(profile.profession)
                          ? profile.profession
                          : "__otro"
                      }
                      onChange={(e) => {
                        if (e.target.value !== "__otro") {
                          handleChange("profession", e.target.value);
                        } else if (
                          PROFESSION_SUGGESTIONS.includes(profile.profession)
                        ) {
                          handleChange("profession", "");
                        }
                      }}
                      className="w-full rounded-lg bg-field border border-border-strong px-3 py-2.5 text-sm text-text-primary focus:border-focus outline-none transition-colors appearance-none"
                    >
                      <option value="" disabled>
                        Seleccioná tu profesión…
                      </option>
                      {PROFESSION_SUGGESTIONS.map((s) => (
                        <option key={s} value={s}>
                          {s}
                        </option>
                      ))}
                      <option value="__otro">Otro…</option>
                    </select>
                    {!PROFESSION_SUGGESTIONS.includes(profile.profession) && (
                      <input
                        id="profile-profession-custom"
                        value={profile.profession}
                        placeholder="Escribí tu profesión"
                        onChange={(e) =>
                          handleChange("profession", e.target.value)
                        }
                        className="w-full rounded-lg bg-field border border-border-strong px-3 py-2.5 text-sm text-text-primary focus:border-focus outline-none transition-colors"
                      />
                    )}
                  </div>
                ) : (
                  <input
                    id={`profile-${item.field}`}
                    value={(profile as any)[item.field]}
                    onChange={(e) => handleChange(item.field, e.target.value)}
                    className="w-full rounded-lg bg-field border border-border-strong px-3 py-2.5 text-sm text-text-primary focus:border-focus outline-none transition-colors"
                  />
                )
              ) : (
                <p
                  id={`profile-${item.field}`}
                  className="text-sm text-text-primary font-medium py-2.5 border-b border-border/50"
                >
                  {(profile as any)[item.field] || "—"}
                </p>
              )}
            </div>
          ))}

          {/* Matrículas profesionales (múltiples: número + jurisdicción) */}
          <div role="group" aria-labelledby="profile-licenses-label">
            <span
              id="profile-licenses-label"
              className="block text-xs font-semibold text-text-muted mb-1.5 uppercase"
            >
              Matrículas Profesionales
            </span>
            {isEditing ? (
              <div className="space-y-2">
                {profile.licenses.map((lic, i) => (
                  <div key={i} className="flex gap-2">
                    <input
                      value={lic.number}
                      aria-label={`Número de matrícula ${i + 1}`}
                      placeholder="Número (ej. MP-12345)"
                      onChange={(e) => handleLicenseChange(i, "number", e.target.value)}
                      className="flex-1 min-w-0 rounded-lg bg-field border border-border-strong px-3 py-2.5 text-sm text-text-primary focus:border-focus outline-none transition-colors"
                    />
                    <input
                      value={lic.jurisdiction}
                      aria-label={`Jurisdicción de matrícula ${i + 1}`}
                      placeholder="Jurisdicción (ej. Córdoba)"
                      onChange={(e) =>
                        handleLicenseChange(i, "jurisdiction", e.target.value)
                      }
                      className="flex-1 min-w-0 rounded-lg bg-field border border-border-strong px-3 py-2.5 text-sm text-text-primary focus:border-focus outline-none transition-colors"
                    />
                    <button
                      type="button"
                      onClick={() => removeLicense(i)}
                      title="Quitar matrícula"
                      aria-label={`Quitar matrícula ${i + 1}`}
                      className="rounded-lg border border-border-strong px-3 text-sm text-text-secondary hover:border-red-700 hover:text-red-400 transition-colors"
                    >
                      <span aria-hidden="true">✕</span>
                    </button>
                  </div>
                ))}
                <div className="flex items-center gap-3 pt-1">
                  <button
                    type="button"
                    onClick={addLicense}
                    className="rounded-lg border border-dashed border-border-strong px-3 py-2 text-xs font-semibold text-text-secondary hover:border-blue-600 hover:text-blue-400 transition-colors"
                  >
                    + Agregar matrícula
                  </button>
                  <span className="text-xs text-text-muted">
                    La primera es la matrícula principal.
                  </span>
                </div>
              </div>
            ) : profile.licenses.length > 0 ? (
              <ul className="space-y-1.5 py-2 border-b border-border/50">
                {profile.licenses.map((lic, i) => (
                  <li key={i} className="text-sm text-text-primary font-medium">
                    {lic.number.trim() || "—"}
                    {lic.jurisdiction.trim() && (
                      <span className="text-text-muted"> — {lic.jurisdiction.trim()}</span>
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
              <p className="text-sm text-text-primary font-medium py-2.5 border-b border-border/50">
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
              <label
                htmlFor={`profile-${item.field}`}
                className="block text-xs font-semibold text-text-muted mb-1.5 uppercase"
              >
                {item.label}
              </label>
              {isEditing ? (
                <input
                  id={`profile-${item.field}`}
                  type={item.type}
                  value={(profile as any)[item.field]}
                  onChange={(e) => handleChange(item.field, e.target.value)}
                  className="w-full rounded-lg bg-field border border-border-strong px-3 py-2.5 text-sm text-text-primary focus:border-focus outline-none transition-colors"
                />
              ) : (
                <p
                  id={`profile-${item.field}`}
                  className="text-sm text-text-primary font-medium py-2.5 border-b border-border/50 truncate"
                >
                  {(profile as any)[item.field] || "—"}
                </p>
              )}
            </div>
          ))}
        </div>
      </div>

      {/* Sección: Seguridad / Cambiar contraseña */}
      <div className="mt-8 pt-6 border-t border-border">
        <ChangePasswordForm />
      </div>

      {isEditing && (
        <div className="mt-8 pt-6 border-t border-border flex justify-end gap-3">
          <button
            onClick={() => setIsEditing(false)}
            className="rounded-lg bg-surface-raised px-6 py-2.5 text-sm font-bold text-text-primary hover:bg-border-strong transition-colors"
          >
            Cancelar
          </button>
          <button
            onClick={handleSave}
            disabled={loading}
            className="rounded-lg bg-emerald-600 px-6 py-2.5 text-sm font-bold text-text-inverse hover:bg-emerald-700 disabled:opacity-50 shadow-lg shadow-emerald-900/20 transition-colors"
          >
            {loading ? "Guardando cambios..." : "Guardar Perfil"}
          </button>
        </div>
      )}
    </div>
  );
}
