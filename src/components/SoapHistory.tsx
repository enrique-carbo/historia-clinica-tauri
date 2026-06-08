interface SoapHistoryProps {
  records: any[];
}

export function SoapHistory({ records }: SoapHistoryProps) {
  if (records.length === 0) {
    return (
      <div
        style={{
          padding: "30px",
          textAlign: "center",
          color: "#71717a",
          border: "1px dashed #27272a",
          borderRadius: "8px",
        }}
      >
        No hay consultas previas registradas para este paciente.
      </div>
    );
  }

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "20px",
        marginTop: "10px",
      }}
    >
      {records.map((record) => (
        <div key={record.id} style={cardStyle}>
          <div style={cardHeaderStyle}>
            <span
              style={{ color: "#3b82f6", fontWeight: "bold", fontSize: "13px" }}
            >
              📅 {new Date(record.created_at).toLocaleString()}
            </span>
            <span style={record.is_synced ? syncBadgeStyle : localBadgeStyle}>
              {record.is_synced ? "Sincronizado" : "Solo Local (Cifrado)"}
            </span>
          </div>

          <div style={gridStyle}>
            <div>
              <strong>S:</strong>{" "}
              <span style={{ color: "#d4d4d8" }}>{record.subjetivo}</span>
            </div>
            <div>
              <strong>O:</strong>{" "}
              <span style={{ color: "#d4d4d8" }}>{record.objetivo}</span>
            </div>
            <div>
              <strong>A:</strong>{" "}
              <span style={{ color: "#d4d4d8" }}>{record.analisis}</span>
            </div>
            <div>
              <strong>P:</strong>{" "}
              <span style={{ color: "#d4d4d8" }}>{record.plan}</span>
            </div>
          </div>
          <div
            style={{
              fontSize: "11px",
              color: "#3f3f46",
              marginTop: "10px",
              fontFamily: "monospace",
            }}
          >
            ID: {record.id}
          </div>
        </div>
      ))}
    </div>
  );
}

const cardStyle: React.CSSProperties = {
  background: "#18181b",
  padding: "20px",
  borderRadius: "8px",
  border: "1px solid #27272a",
};
const cardHeaderStyle: React.CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "center",
  borderBottom: "1px solid #27272a",
  paddingBottom: "10px",
  marginBottom: "12px",
};
const gridStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: "8px",
  fontSize: "14px",
};
const localBadgeStyle: React.CSSProperties = {
  fontSize: "11px",
  background: "#2e1065",
  color: "#c084fc",
  padding: "2px 8px",
  borderRadius: "12px",
  fontWeight: "600",
};
const syncBadgeStyle: React.CSSProperties = {
  fontSize: "11px",
  background: "#062f17",
  color: "#4ade80",
  padding: "2px 8px",
  borderRadius: "12px",
  fontWeight: "600",
};
