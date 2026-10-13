// src/stores/useEntityStore.test.ts

import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { useEntityStore, type Entity } from "./useEntityStore";
import { useAuthStore } from "./useAuthStore";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);

const ENTITY: Entity = {
  id: 1,
  entity_type: "patient",
  data: { nombre: "Ana", apellido: "Gómez" },
  created_at: "2026-01-01T00:00:00Z",
};

const OLDER: Entity = { ...ENTITY, id: 2, data: { nombre: "Bruno" } };

beforeEach(() => {
  vi.clearAllMocks();
  useEntityStore.setState({
    entities: [],
    selectedEntity: null,
    isLoading: false,
    error: null,
  });
  useAuthStore.setState({ isVaultUnlocked: false, activeUser: null });
});

describe("fetchEntities", () => {
  it("lista entities y limpia loading/error", async () => {
    invokeMock.mockResolvedValueOnce([ENTITY]);
    await useEntityStore.getState().fetchEntities("patient");

    const s = useEntityStore.getState();
    expect(s.entities).toEqual([ENTITY]);
    expect(s.isLoading).toBe(false);
    expect(s.error).toBeNull();
    expect(invokeMock).toHaveBeenCalledWith("list_entities", {
      entityType: "patient",
      limit: 50,
      offset: 0,
    });
  });

  it("respeta el limit pasado y expone el error del backend", async () => {
    invokeMock.mockRejectedValueOnce("boom");
    await useEntityStore.getState().fetchEntities("patient", 10);

    const s = useEntityStore.getState();
    expect(s.entities).toEqual([]);
    expect(s.isLoading).toBe(false);
    expect(s.error).toBe("boom");
    expect(invokeMock).toHaveBeenCalledWith("list_entities", {
      entityType: "patient",
      limit: 10,
      offset: 0,
    });
  });
});

describe("searchEntities", () => {
  it("query con contenido invoca search_entities", async () => {
    invokeMock.mockResolvedValueOnce([ENTITY]);
    await useEntityStore.getState().searchEntities("patient", "gómez");

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("search_entities", {
      entityType: "patient",
      query: "gómez",
    });
    expect(useEntityStore.getState().entities).toEqual([ENTITY]);
  });

  it("query vacía (o solo espacios) vuelve al listado general", async () => {
    invokeMock.mockResolvedValueOnce([OLDER]);
    await useEntityStore.getState().searchEntities("patient", "   ");

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith("list_entities", {
      entityType: "patient",
      limit: 50,
      offset: 0,
    });
  });

  it("expone el error del backend", async () => {
    invokeMock.mockRejectedValueOnce("fallo búsqueda");
    await useEntityStore.getState().searchEntities("patient", "x");

    expect(useEntityStore.getState().error).toBe("fallo búsqueda");
  });
});

describe("createEntity", () => {
  it("rechaza sin sesión activa y no invoca al backend", async () => {
    const res = await useEntityStore.getState().createEntity({ nombre: "Ana" });

    expect(res).toBeNull();
    expect(invokeMock).not.toHaveBeenCalled();
    expect(useEntityStore.getState().error).toBe("Usuario no autenticado");
  });

  it("crea, prepend en la lista y selecciona (optimistic UI)", async () => {
    useAuthStore.setState({
      isVaultUnlocked: true,
      activeUser: { user_id: "u1", username: "dra", role: "profesional" },
    });
    useEntityStore.setState({ entities: [OLDER] });
    invokeMock.mockResolvedValueOnce(ENTITY);

    const res = await useEntityStore
      .getState()
      .createEntity({ nombre: "Ana" });

    expect(res).toEqual(ENTITY);
    expect(invokeMock).toHaveBeenCalledWith("create_entity", {
      entityType: "patient",
      data: { nombre: "Ana" },
    });
    const s = useEntityStore.getState();
    expect(s.entities).toEqual([ENTITY, OLDER]);
    expect(s.selectedEntity).toEqual(ENTITY);
    expect(s.isLoading).toBe(false);
  });

  it("error del backend deja null y expone el mensaje", async () => {
    useAuthStore.setState({
      isVaultUnlocked: true,
      activeUser: { user_id: "u1", username: "dra", role: "profesional" },
    });
    invokeMock.mockRejectedValueOnce("dni duplicado");

    const res = await useEntityStore.getState().createEntity({ dni: "1" });

    expect(res).toBeNull();
    const s = useEntityStore.getState();
    expect(s.error).toBe("dni duplicado");
    expect(s.isLoading).toBe(false);
    expect(s.entities).toEqual([]);
  });
});

describe("findByBlindIndex / selectEntity / clearError", () => {
  it("findByBlindIndex devuelve el id o null con error", async () => {
    invokeMock.mockResolvedValueOnce(7);
    expect(
      await useEntityStore.getState().findByBlindIndex("patient", "123"),
    ).toBe(7);

    invokeMock.mockRejectedValueOnce("no encontrado");
    expect(
      await useEntityStore.getState().findByBlindIndex("patient", "999"),
    ).toBeNull();
    expect(useEntityStore.getState().error).toBe("no encontrado");
  });

  it("selectEntity setea y clearError limpia", () => {
    useEntityStore.getState().selectEntity(ENTITY);
    expect(useEntityStore.getState().selectedEntity).toEqual(ENTITY);

    useEntityStore.setState({ error: "x" });
    useEntityStore.getState().clearError();
    expect(useEntityStore.getState().error).toBeNull();
  });
});
