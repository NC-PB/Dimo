# Exporte

## Aus der App exportieren

Öffne die Ansicht **Export** (die Schaltfläche **Export** in der oberen Leiste, oder ⌘E auf macOS,
Strg+E sonst). Sie braucht ein offenes Projekt. Jeder Export fragt, wo die Datei gespeichert wird,
und läuft dann im Hintergrund; du kannst währenddessen weiterarbeiten. Die Datei enthält das
Projekt so, wie es war, als du die Schaltfläche geklickt hast. Unter dem Export erscheinen ein
Fortschrittsbalken und danach der Dateiname, oder der Grund, warum der Export fehlschlug.

Die vorgeschlagenen Dateinamen folgen der Projektdatei (bei einem nie gespeicherten Projekt der
Zeichnung): `teil_ballooned.pdf`, `teil_characteristics.csv` und `teil_characteristics.xlsx`.

Optionen. Die ersten beiden werden für den nächsten Start gespeichert:

- **Spaltenüberschriften**: englische oder deutsche Überschriften in den CSV- und Excel-Dateien,
  unabhängig von der Sprache der App.
- **Ballons im PDF**: als Teil der Seite (Standard) oder als Anmerkungen, die PDF-Programme
  ein- und ausblenden oder löschen können.
- **Als freigegeben exportieren**: sperrt die Nummerierung vor dem Export. Für Zeichnungen, die
  an einen Kunden gehen. Solange sie gesperrt ist, ändern sich Nummern nie, gelöschte Nummern
  werden nicht wieder vergeben, ein neues Merkmal erhält die höchste je vergebene Nummer plus
  eins, und Zeilen der Tabelle lassen sich nicht verschieben. Die Sperre steht im
  Änderungsprotokoll. Dieses Feld wird nicht gespeichert und ist bei jedem Start aus. Solange die
  Nummerierung gesperrt ist, ist das Feld deaktiviert und sagt das auch.

Eine Schaltfläche zum Entsperren gibt es in dieser Version nicht. Die Sperre ist eine Änderung wie
jede andere, ⌘Z macht sie also rückgängig, solange sie deine letzte Änderung ist, zum Beispiel
direkt nach dem Export. Was du nach dem Export änderst, liegt im Verlauf darüber.

## PDF mit Ballons

Eine Kopie der Zeichnung mit den Ballons als Vektorgrafik. Die Originaldatei bleibt unverändert.
Die Ballons sehen aus wie in der App: gleiche Form, Größe, Farben und Bezugslinien. Merkmale mit
dem Status `rejected` erhalten im PDF keinen Ballon.

Dasselbe Projekt ergibt dieselbe PDF-Datei, Byte für Byte. Bei Anmerkungen ist deren Datum der
Zeitpunkt der letzten Änderung am Projekt, nicht der Zeitpunkt des Exports.

## Merkmalsliste (CSV und XLSX)

Die Merkmalsliste enthält eine Zeile pro Merkmal, sortiert nach Ballonnummer. Sie ist für die
Programmierung eines Koordinatenmessgeräts (CSV) und für die Arbeit in einer Tabellenkalkulation
(XLSX) gedacht. Beide Dateien haben dieselben Spalten.

Merkmale mit dem Status `rejected` werden nicht exportiert. Merkmale, bei denen du **Prüfen**
abgewählt hast, etwa Hilfsmaße (Referenzmaße) und theoretisch genaue Maße (Rahmenmaße), werden mit
`Prüfen` gleich `no` exportiert und lassen sich so ausfiltern.

Die Sprache der Spaltenüberschriften wählst du pro Export: Englisch oder Deutsch. Die Werte
ändern sich mit der Sprache nicht.

### Spalten

| Spalte (Deutsch) | Spalte (Englisch) | Inhalt |
|---|---|---|
| Nr | No | Ballonnummer, z. B. `12`, oder `12.1` und `12A` für Merkmale, die bei gesperrter Nummerierung hinzukommen. Einfache Nummern sind in XLSX Zahlenzellen, die anderen Text |
| Art | Kind | `linear`, `diameter`, `radius`, `spherical_radius`, `angle`, `chamfer`, `thread`, `counterbore`, `countersink`, `depth`, `surface_texture`, `geometric`, `note`, `flag_note`, `material_process`, `other` |
| Anforderung | Requirement | Text wie auf der Zeichnung |
| Nennmaß | Nominal | Nennwert |
| Oberes Abmaß | Upper deviation | Oberes Abmaß mit Vorzeichen |
| Unteres Abmaß | Lower deviation | Unteres Abmaß mit Vorzeichen |
| Obere Grenze | Upper limit | Oberes Grenzmaß, absolut |
| Untere Grenze | Lower limit | Unteres Grenzmaß, absolut |
| Einheit | Unit | `mm`, `in` oder `deg` |
| Passung | Fit | Passungskurzzeichen, z. B. `H7` |
| Regel | Rule | Regel, die die Grenzmaße ergab: `explicit`, `fit`, `drawing_rule`, `general`, `decimal_rule`, `custom_table`, `no_tolerance_defined` oder `manual` (von Hand eingegeben); leer, wenn keine erfasst ist ([Toleranzen](tolerances.md)) |
| Anzahl | Quantity | Anzahl der Elemente, für die das Merkmal steht, aus der Spalte Anzahl |
| Klassifizierung | Classification | `critical`, `major`, `minor` oder `key`; leer ohne Klassifizierung |
| Prüfmethode | Inspection method | Freitext |
| Prüfmittel | Gauge | Freitext |
| Stichprobe | Sampling | Freitext |
| Häufigkeit | Frequency | Freitext |
| Prüfen | Inspect | `yes` oder `no` |
| Status | Status | `proposed`, `accepted` oder `verified` |
| Blatt | Sheet | Seitennummer des Blatts, ab 1. Leer, wenn das Merkmal weder einen Bereich noch einen Ballon hat |
| Kommentar | Comment | Freitext |

Ein nicht gesetzter Wert ist ein leeres Feld beziehungsweise eine leere Zelle.

### Zahlen sind exakt

Nennmaße, Abmaße und Grenzen werden mit genau den im Projekt gespeicherten Ziffern geschrieben.
`90.0` bleibt `90.0` und `30.0203` bleibt `30.0203`. Das Dezimaltrennzeichen ist immer ein Punkt.

- CSV: Text wie `-0.2`. Es wird nichts gerundet oder umgewandelt.
- XLSX: Ein Wert mit bis zu 15 signifikanten Stellen ist eine Zahlenzelle, angezeigt mit der
  gespeicherten Anzahl Nachkommastellen. Ein Wert mit mehr Stellen wird als Text geschrieben,
  weil eine Tabellenzahl ihn nicht exakt aufnehmen kann. Solche Zellen sind linksbündig.

### CSV-Format

UTF-8 ohne Byte-Order-Mark, Komma als Trennzeichen, eine Kopfzeile, Zeilenende `\n`. Ein Feld
steht in doppelten Anführungszeichen, wenn es ein Komma, ein Anführungszeichen oder einen
Zeilenumbruch enthält; Anführungszeichen im Text werden verdoppelt (RFC 4180). Anforderungstexte
und Kommentare werden unverändert geschrieben. Öffnest du die CSV in einer Tabellenkalkulation,
prüfe Texte, die mit `=`, `+`, `-` oder `@` beginnen, bevor du sie erneut speicherst. Deutsche
Excel-Versionen erkennen UTF-8 ohne Byte-Order-Mark beim Doppelklick nicht immer; importiere die
Datei dann über "Daten aus Text/CSV" mit der Kodierung UTF-8.

### XLSX-Format

Ein Tabellenblatt (`Merkmale` oder `Characteristics`) mit fetter Kopfzeile, fixierter Kopfzeile
und Spaltenfiltern.

### Identische Dateien

Dasselbe Projekt, dieselbe Version und dieselben Optionen ergeben byte-identische Dateien. Die
XLSX enthält keine Erstellzeit, keinen Benutzernamen und keinen anderen Wert, der davon abhängt,
wann oder wo exportiert wird.
