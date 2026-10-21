"""Safe PerchFlow local-test fixture: uses only this directory, no internet or accounts."""
from pathlib import Path
import csv
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parent
# A real Python file is required, but this demo contains NO credentials.
fixture = ROOT / "input.txt"
if not fixture.is_file():
    raise SystemExit("Required dummy input.txt is missing")
value = fixture.read_text(encoding="utf-8").strip()
output = ROOT / "output"
output.mkdir(exist_ok=True)
stamp = datetime.now(timezone.utc).isoformat()
with (output / "report.csv").open("w", newline="", encoding="utf-8") as stream:
    writer = csv.writer(stream)
    writer.writerow(["kind", "value", "created_at"])
    writer.writerow(["MOCK_ONLY", value, stamp])
(ROOT / "results.txt").write_text(
    "MOCK_ONLY successful run at " + stamp + "\n", encoding="utf-8"
)
print("MOCK_ONLY Python task completed; demo output files were generated.")
