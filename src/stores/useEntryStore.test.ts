// src/stores/useEntryStore.test.ts

import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { useEntryStore, type Entry } from "./useEntryStore";
import { useAuthStore } from "./useAuthStore";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const invokeMock = vi.mocked(invoke);

function makeEntry(id: string): Entry {
  return {
    id,
    category: "SOAP_NOTE",
    subject_id: 1,
    author_id: "u1",
    author_name: "Dra. Pérez",
    author_profession: "Médico",
    title: `Entry ${id}`,
    status: "ACTIVE",
    timestamp: "2026-01-01T00:00:00Z",
    payload: { subjetivo: "x" },
    signature: "sig",
    is_verified: true,
    created_at: "2026-01-01T00:00:00Z",
  };
}

const PAGE_SIZE = 20;
const fullPage = (prefix: string) =>
  Array.from({ length: PAGE_SIZE }, (_, i) => makeEntry(`${prefix}${i}`));
const shortPage = (prefix: string, n: number) =>
  Array.from({ length: n }, (_, i) => makeEntry(`${prefix}${i}`));

beforeEach(() => {
  vi.clearAllMocks();
  useEntryStore.setState({
    entries: [],
    selectedEntry: null,
    isLoading: false,
    isLoadingMore: false,
    error: null,
    page: 0,
    pageSize: PAGE_SIZE,
    hasMore: true,
  });
  useAuthStore.setState({ isVaultUnlocked: false, activeUser: null });
});

describe("fetchEntries", () => {
  it("carga la primera página y setea hasMore según el largo", async () => {
    invokeMock.mockResolvedValueOnce(fullPage("a"));
    await useEntryStore.getState().fetchEntries(1);

    const s = useEntryStore.getState();
    expect(s.entries).toHaveLength(PAGE_SIZE);
    expect(s.page).toBe(1);
    expect(s.hasMore).toBe(true);
    expect(s.isLoading).toBe(false);
    expect(s.isLoadingMore).toBe(false);
    expect(invokeMock).toHaveBeenCalledWith("get_entries_by_subject", {
      subjectId: 1,
      limit: PAGE_SIZE,
      offset: 0,
    });
  });

  it("página corta implica hasMore=false; con categoría invoca el comando filtrado", async () => {
    invokeMock.mockResolvedValueOnce(shortPage("b", 3));
    await useEntryStore.getState().fetchEntries(1, "ALLERGY");

    const s = useEntryStore.getState();
    expect(s.entries).toHaveLength(3);
    expect(s.hasMore).toBe(false);
    expect(invokeMock).toHaveBeenCalledWith("get_entries_by_category", {
      subjectId: 1,
      category: "ALLERGY",
      limit: PAGE_SIZE,
      offset: 0,
    });
  });

  it("expone el error del backend", async () => {
    invokeMock.mockRejectedValueOnce("boom");
    await useEntryStore.getState().fetchEntries(1);

    const s = useEntryStore.getState();
    expect(s.error).toBe("boom");
    expect(s.isLoading).toBe(false);
  });
});

describe("loadMore", () => {
  it("append página siguiente, incrementa page y no toca isLoading (sin parpadeo)", async () => {
    invokeMock.mockResolvedValueOnce(fullPage("a"));
    await useEntryStore.getState().fetchEntries(1);

    invokeMock.mockResolvedValueOnce(shortPage("c", 2));
    await useEntryStore.getState().loadMore(1);

    const s = useEntryStore.getState();
    expect(s.entries).toHaveLength(PAGE_SIZE + 2);
    expect(s.page).toBe(2);
    expect(s.hasMore).toBe(false);
    // El flag de carga inicial queda intacto: la paginación usa isLoadingMore.
    expect(s.isLoading).toBe(false);
    expect(s.isLoadingMore).toBe(false);
    expect(invokeMock).toHaveBeenLastCalledWith("get_entries_by_subject", {
      subjectId: 1,
      limit: PAGE_SIZE,
      offset: PAGE_SIZE,
    });
  });

  it("con query delega en search_entries con el offset correcto", async () => {
    invokeMock.mockResolvedValueOnce(fullPage("a"));
    await useEntryStore.getState().searchEntries(1, "gripe");

    invokeMock.mockResolvedValueOnce(shortPage("d", 1));
    await useEntryStore.getState().loadMore(1, undefined, "gripe");

    expect(invokeMock).toHaveBeenLastCalledWith("search_entries", {
      subjectId: 1,
      query: "gripe",
      category: null,
      limit: PAGE_SIZE,
      offset: PAGE_SIZE,
    });
  });

  it("no hace nada si no hay más páginas", async () => {
    invokeMock.mockResolvedValueOnce(shortPage("e", 1));
    await useEntryStore.getState().fetchEntries(1);

    await useEntryStore.getState().loadMore(1);

    expect(invokeMock).toHaveBeenCalledTimes(1);
  });

  it("error del backend deja isLoadingMore en false", async () => {
    invokeMock.mockResolvedValueOnce(fullPage("a"));
    await useEntryStore.getState().fetchEntries(1);

    invokeMock.mockRejectedValueOnce("fallo paginación");
    await useEntryStore.getState().loadMore(1);

    const s = useEntryStore.getState();
    expect(s.error).toBe("fallo paginación");
    expect(s.isLoadingMore).toBe(false);
    expect(s.page).toBe(1);
  });
});

describe("createEntry", () => {
  it("rechaza sin sesión activa y no invoca al backend", async () => {
    const res = await useEntryStore.getState().createEntry({
      category: "SOAP_NOTE",
      subjectId: 1,
      title: "t",
      status: "ACTIVE",
      payload: {},
    });

    expect(res).toBeNull();
    expect(invokeMock).not.toHaveBeenCalled();
    expect(useEntryStore.getState().error).toBe("Usuario no autenticado");
  });

  it("crea, recupera la entry firmada y la prepend en la lista", async () => {
    useAuthStore.setState({
      isVaultUnlocked: true,
      activeUser: { user_id: "u1", username: "dra", role: "profesional" },
    });
    const created = makeEntry("n1");
    invokeMock
      .mockResolvedValueOnce("n1")
      .mockResolvedValueOnce(created);

    const res = await useEntryStore.getState().createEntry({
      category: "SOAP_NOTE",
      subjectId: 1,
      title: "Nota",
      status: "ACTIVE",
      payload: { subjetivo: "s" },
    });

    expect(res).toEqual(created);
    expect(invokeMock).toHaveBeenNthCalledWith(1, "create_entry", {
      category: "SOAP_NOTE",
      subjectId: 1,
      title: "Nota",
      status: "ACTIVE",
      payload: { subjetivo: "s" },
    });
    expect(invokeMock).toHaveBeenNthCalledWith(2, "get_entry", { id: "n1" });
    expect(useEntryStore.getState().entries).toEqual([created]);
  });
});

describe("selectEntry / clearError", () => {
  it("selectEntry setea y clearError limpia", () => {
    const entry = makeEntry("s1");
    useEntryStore.getState().selectEntry(entry);
    expect(useEntryStore.getState().selectedEntry).toEqual(entry);

    useEntryStore.setState({ error: "x" });
    useEntryStore.getState().clearError();
    expect(useEntryStore.getState().error).toBeNull();
  });
});
