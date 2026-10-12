# Exporte

## Aus der App exportieren

Öffne die Ansicht **Export** (die Schaltfläche **Export** in der oberen Leiste, oder ⌘E auf macOS,
Strg+E sonst). Sie braucht ein offenes Projekt. Jeder Export fragt, wo die Datei gespeichert wird,
und läuft dann im Hintergrund; du kannst währenddessen weiterarbeiten. Die Datei enthält das
Projekt so, wie es war, als du die Schaltfläche geklickt hast. Unter dem Export erscheinen ein
Fortschrittsbalken und danach der Dateiname, oder der Grund, warum der Export fehlschlug.
Während ein Bericht exportiert wird, hält **Export abbrechen** ihn an; dann wird keine Datei
geschrieben. Dimo hält zwischen den Schritten des Exports an (vorbereiten, erzeugen, schreiben),
das Abbrechen kann also einen Moment dauern.

Die Ansicht hat zwei Gruppen: **Berichte** (Prüfblatt und Prüfbericht) und **Zeichnung und
Merkmalsliste** (PDF mit Ballons, CSV- und Excel-Liste).

Die vorgeschlagenen Dateinamen folgen der Projektdatei (bei einem nie gespeicherten Projekt der
Zeichnung): `teil_ballooned.pdf`, `teil_characteristics.csv`, `teil_characteristics.xlsx`,
`teil_check-sheet.pdf` oder `.xlsx` und `teil_inspection-report.pdf` oder `.xlsx`.

Optionen. Alle außer der letzten werden für den nächsten Start gespeichert, ebenso das für jeden
Bericht gewählte Format:

- **Sprache**: englische oder deutsche Überschriften in den CSV- und Excel-Dateien und die
  Sprache der Berichte, unabhängig von der Sprache der App.
- **Ballons im PDF**: als Teil der Seite (Standard) oder als Anmerkungen, die PDF-Programme
  ein- und ausblenden oder löschen können.
- **Leere Ergebnisspalten**: wie viele leere Ergebnisspalten ein Prüfblatt erhält, wenn das
  Projekt keine Muster hat, 1 bis 10 (Standard 3).
- **Als freigegeben exportieren**: sperrt die Nummerierung. Für Zeichnungen und Berichte, die
  an einen Kunden gehen. Solange sie gesperrt ist, ändern sich Nummern nie, gelöschte Nummern
  werden nicht wieder vergeben, ein neues Merkmal erhält die höchste je vergebene Nummer plus
  eins, und Zeilen der Tabelle lassen sich nicht verschieben. Die Sperre steht im
  Änderungsprotokoll. Ein als freigegeben exportierter Bericht wird außerdem im Projekt vermerkt
  (siehe [Freigegebene Berichte](#freigegebene-berichte)). Dieses Feld wird nicht gespeichert und
  ist bei jedem Start aus. Solange die Nummerierung gesperrt ist, sagt das Feld das; als
  freigegeben exportierte Berichte werden trotzdem vermerkt.

Eine Schaltfläche zum Entsperren gibt es in dieser Version nicht. Die Sperre ist eine Änderung wie
jede andere, ⌘Z macht sie also rückgängig, solange sie deine letzte Änderung ist, zum Beispiel
direkt nach dem Export. Was du nach dem Export änderst, liegt im Verlauf darüber.

## Prüfblatt und Prüfbericht

Beide Berichte führen die zu prüfenden Merkmale in Nummernfolge auf: Merkmale ohne **Prüfen**
(Referenz- und theoretisch genaue Maße) und abgelehnte fehlen. Jede Zeile zeigt Nummer,
Anforderung, Nennmaß, Grenzen, Einheit, die Regel, aus der die Grenzen stammen, Prüfmethode und
Prüfmittel. Der Kopf jeder Seite zeigt Firma, Teil, Kunde, Auftrag, Zeichnungsdatei,
Änderungsstand, den Anfang des SHA-256 der Zeichnungsdatei (vollständig auf der ersten Seite),
Berichtsnummer, Datum und Seite. Die Werte stammen aus
[Projekt- und Berichtsdaten](project-details.md).

- **Prüfblatt**: leere Ergebnisspalten zum Ausdrucken und Ausfüllen von Hand und eine Zeile für
  den Prüfer. Hat das Projekt Muster, gibt es eine Spalte je Muster mit seiner Seriennummer; ohne
  Muster die in den Optionen eingestellte Zahl leerer Ergebnisspalten, mit dem Hinweis, die
  Seriennummer jedes Teils in den Spaltenkopf zu schreiben. Das Prüfblatt zeigt keine
  Bewertungen.
- **Prüfbericht**: eine Übersicht (Gesamtergebnis, Anzahl der Merkmale und Muster sowie Anzahl
  i.O., n.i.O., nicht bewertet und fehlend), ein Musterblock mit einer Zeile je Muster
  (Seriennummer, Datum, Prüfer, Prüfmittel, Bemerkung, seine eigenen Zahlen und seine Bewertung),
  die Ergebnistabelle mit dem Zeichen jeder Bewertung und der Bewertung jeder Zeile, eine Legende
  der Zeichen, Bemerkungen und der Unterschriftenblock. Wie die Bewertungen entstehen, steht unter
  [Ergebnisse eingeben](measuring.md).

Der Unterschriftenblock hat zwei Zeilen, **Geprüft von** und **Freigegeben von**, jeweils mit Name,
Datum und einem leeren Feld für die handschriftliche Unterschrift. Die Namen kommen aus den
Berichtsfeldern. Das Datum bei „Geprüft von“ ist das Berichtsdatum; das Datum bei „Freigegeben von“
bleibt leer und wird von Hand eingetragen. Digitale Signaturen gibt es nicht. Ist das
Gesamtergebnis offen, sagt der Bericht, dass nicht alle Ergebnisse erfasst und bewertet sind.
Nicht bestandene Zellen und Muster haben einen fetten Rand.

Wähle unter **Format** jedes Berichts **PDF** oder **Excel**. Beide enthalten dieselben Werte.

- **PDF**: A4 quer, Kopf auf jeder Seite, Tabellenkopf auf jeder Seite wiederholt. Bei mehr als
  sechs Mustern wird die Tabelle in Blöcken von sechs Musterspalten gedruckt, jeder Block auf
  einer neuen Seite, mit wiederholten Merkmalsspalten und einer Überschrift wie „Teile 7 bis 12
  von 12“.
- **Excel**: ein Blatt mit dem Namen des Berichts. Werte sind Zahlenzellen mit den eingegebenen
  Stellen; das Zeichen (✓ ✗ ○) gehört zum Zellformat, die Zelle bleibt also eine Zahl.
  Bewertungen und i.O.-/n.i.O.-Ergebnisse sind Text mit dem Zeichen. Die Zellen haben zusätzlich
  eine Füllfarbe (grün, rot, gelb, grau für offen). Die Spalten A und B sind fixiert. Das Blatt ist
  für A4 quer eingerichtet, mit auf jeder gedruckten Seite wiederholten Kopfzeilen; bei mehr als
  sechs Mustern wird mit 60 Prozent gedruckt, mit einem Seitenumbruch nach jeweils sechs
  Musterspalten und wiederholten Merkmalsspalten. Der Prüfbericht enthält für jedes Muster die
  Anzahl i.O., n.i.O., nicht bewertet und fehlend, wie das PDF.

Die Wörter auf den Berichten richten sich nach der Berichtssprache des Exports (Englisch oder
Deutsch), nicht nach der Sprache der App. Auf Deutsch heißen die Bewertungen **i.O.** und
**n.i.O.**, auf Englisch **Pass** und **Fail**. In den deutschen Berichten heißen die Muster
„Teile“, und der Revisionsstand heißt „Änderungsstand“.

Grenzen aus Toleranztabellen, die noch Entwürfe sind, sind in der Regelspalte mit einem Stern
markiert, und der Bericht sagt in einer Fußnote, dass diese Grenzen aus einer Entwurfstabelle
stammen und noch nicht verifiziert sind.

Das Datum auf einem Bericht ist das Berichtsdatum des Projekts. Ist es leer, gilt das Datum der
letzten Änderung am Projekt. Es hängt nie vom Tag des Exports ab, dasselbe Projekt ergibt also
dieselben Dateien, Byte für Byte.

### Freigegebene Berichte

Mit **Als freigegeben exportieren** erledigt ein Berichtsexport nach dem Schreiben der Datei drei
Dinge in einem Schritt:

1. Die Nummerierung wird gesperrt, falls sie es noch nicht ist.
2. Ein leerer Firmenname wird aus den **Einstellungen** ergänzt und ein leeres „geprüft von“ mit
   deinem Benutzernamen. Der Bericht zeigt diese Werte bereits. Ein normaler Export druckt das
   Projekt, wie es ist, und lässt leere Felder leer.
3. Das Projekt vermerkt den freigegebenen Bericht: welcher Bericht, die Vorlage und ihre Version,
   das Dateiformat, den Änderungsstand der Zeichnung und deren SHA-256, den SHA-256 der
   geschriebenen Datei und den Exportzeitpunkt. Der Exportzeitpunkt steht nur im Projekt, nicht im
   Bericht.

Das Vermerken ist eine Änderung am Projekt. Ein Bericht ohne Berichtsdatum trägt deshalb bei einem
erneuten Export ein späteres Datum; gib ein Berichtsdatum ein, damit es fest bleibt.

⌘Z macht alle drei auf einmal rückgängig. Unter dem Export bestätigt Dimo „Als freigegeben
vermerkt“. Hast du das Projekt während des Exports geändert, wird die Datei geschrieben, aber
nichts vermerkt, weil die Datei das Projekt nicht mehr zeigt; exportiere dann erneut.

Vor einem Export als freigegeben warnt Dimo, wenn Grenzen aus Entwurfstabellen stammen, wenn
Ergebnisse fehlen oder nicht bewertet sind (Prüfbericht) oder wenn sich Grenzen oder
Ergebnisarten nach dem letzten freigegebenen Bericht geändert haben
([Wenn sich Grenzmaße ändern](measuring.md#wenn-sich-grenzmaße-ändern)). Du kannst trotzdem
exportieren oder zurückgehen. Die Warnungen verhindern den Export nie. Beide Berichte können
freigegeben werden, als PDF und als Excel; jede freigegebene Datei wird für sich vermerkt.

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
