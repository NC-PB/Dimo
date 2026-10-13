# Eine Zeichnung ansehen

Auf der Zeichnung arbeitest du. Sie zeigt das Blatt in der Mitte, die Merkmalstabelle darunter
([Merkmalstabelle](characteristics.md)) und den Eigenschaftenbereich rechts.

## Das Fenster

- **Titelleiste**: Speichern, Rückgängig und Wiederholen, der Name des Projekts mit "gespeichert"
  oder "nicht gespeichert", der Schalter hell oder dunkel und **Hilfe** (die Liste der
  Tastenkürzel).
- **Reiter**: **Datei** (Projektinfo, neu, öffnen mit den zuletzt geöffneten Projekten, speichern, speichern unter, schließen, Optionen, über Dimo), **Start**
  (Ballons setzen und bearbeiten, Nummerierung, Allgemeintoleranz), **Zeichnung** (Drehung,
  Einheit, Maßstab, Zonenraster, Ansichten, Toleranzen, Projektinfo), **Überprüfen** (in dieser
  Version ein Platzhalter), **Messen** ([Ergebnisse eingeben](measuring.md)), **Bericht**
  ([Exporte](exports.md)) und **Ansicht** (Zoom, Tabelle und Eigenschaftenbereich zeigen oder verbergen,
  Darstellung). Jeder Reiter zeigt seine Befehle in einer Zeile unter den Reitern. Halte den
  Zeiger über einen Befehl, um zu sehen, was er tut, sein Tastenkürzel und, wenn er grau ist,
  warum er gerade nicht geht. In einem schmalen Fenster rutschen die letzten Gruppen unter
  **Mehr**.
- **Statusleiste** unten: die Blätter, das aktive Werkzeug, die Anzahl Merkmale und ausgewählte
  (im Reiter Messen die Summen aller Teile), die Allgemeintoleranz, Einheit und Maßstab, und
  der Zoom.

Start und Zeichnung zeigen beide die Zeichnung (⌘1). ⌘2 zeigt Überprüfen, ⌘3 Messen, ⌘E Bericht.

## Zoomen

| Aktion | So geht es |
|---|---|
| Um den Zeiger vergrößern oder verkleinern | Mausrad, oder Spreizen und Zusammenziehen auf dem Trackpad |
| Um die Mitte vergrößern oder verkleinern | **+** und **-**, die Schaltflächen im Reiter Ansicht oder der Zoom in der Statusleiste |
| Das ganze Blatt ins Fenster einpassen | **0**, oder **Blatt einpassen** im Reiter Ansicht |

Die Statusleiste zeigt den Zoom in Prozent der gedruckten Größe: 100 % heißt, die Zeichnung ist
auf dem Bildschirm so groß wie auf Papier. Ballons behalten ihre gedruckte Größe, sie wachsen und
schrumpfen also mit dem Zoom.

## Die Zeichnung verschieben

| Aktion | So geht es |
|---|---|
| In kleinen Schritten verschieben | Pfeiltasten |
| Die Zeichnung ziehen | Mittlere Maustaste, oder Leertaste gedrückt halten und mit der linken Taste ziehen |
| Die Zeichnung mit dem Werkzeug Auswählen ziehen | Auf leerer Zeichnung ziehen |

Die Pfeiltasten verschieben die Ansicht nur, solange die Zeichnung, nicht ein Textfeld oder die
Tabelle, den Tastaturfokus hat. Shift mit einer Pfeiltaste verschiebt stattdessen die
ausgewählten Ballons ([Eine Zeichnung ballonieren](ballooning.md)). Einzelne Tasten wie **+**,
**-**, **0**, **R**, **B**, **V** und **S** wirken nicht auf die Zeichnung, solange die Tabelle den
Fokus hat.

## Blätter

Ein PDF mit mehreren Seiten hat ein Blatt pro Seite. Die Statusleiste zeigt einen Reiter pro
Blatt ("Blatt 2"); ein Klick zeigt es an. Auch **Bild auf** und **Bild ab** wechseln die Blätter.
Ein Blatt wird beim ersten Anzeigen ins Fenster eingepasst. Wählst du in der
Tabelle ein Merkmal, dessen Ballon auf einem anderen Blatt liegt, wechselt Dimo zu diesem Blatt und
zeigt den Ballon.

Jedes Blatt hat eigene Drehung, Einheit und einen eigenen Maßstab ([Blätter](sheets.md)). Das Drehen
eines Blatts dreht die Ansicht: Ballons und ihre Nummern bleiben aufrecht, und Exporte sind nicht
betroffen.

## Eigenschaftenbereich

Der Bereich rechts folgt der Auswahl. Sein Kopf sagt, was er zeigt; das × blendet ihn aus
(**Ansicht > Eigenschaften** zeigt ihn wieder). Ein Klick auf die Überschrift eines Abschnitts
klappt ihn zu oder auf; Dimo merkt sich, welche Abschnitte du zugeklappt hast.

- **Nichts ausgewählt**: eine Übersicht des Projekts (Teil, Zeichnung, Allgemeintoleranz, Anzahl
  Merkmale und Blätter) mit **Unter Datei > Info bearbeiten**, wo du diese Werte änderst
  ([Projekt- und Berichtsdaten](project-details.md)), und ein Hinweis zum Start.
- **Ein Merkmal**: **Anforderung** (Text, Art, Anzahl), **Toleranz** (Regel, Grenzmaße, der kürzere
  Schenkel eines Winkels, die Erklärung und **Grenzmaße aktualisieren**), **Prüfung** (prüfen,
  Ergebnisart, Prüfmethode, Prüfmittel) und **Verlauf** ([Toleranzen](tolerances.md)). Jede
  Änderung ist ein Rückgängig-Schritt, wie eine Änderung in der Tabelle.
- **Mehrere Merkmale**: die Liste, **Grenzmaße aktualisieren** für alle und **Ja** oder **Nein**, um
  alle zu prüfen oder nicht.
- **Zonenraster** und **Ansicht zeichnen** auf dem Reiter Zeichnung öffnen einen Abschnitt, in dem
  du das Raster oder die Liste der Ansichten bearbeitest, bis du ihn mit seinem × schließt
  ([Nummerierungsstrategien, Zonen und Ansichten](numbering.md)).
- Solange **Nummern-Vorschau** eingeschaltet ist, zählt ein Abschnitt die Nummern, die sich
  ändern.

Drehung, Einheit und Maßstab eines Blatts stehen auf dem Reiter Zeichnung ([Blätter](sheets.md)),
die Reihenfolge der Nummerierung auf dem Reiter Start.

## Höhe der Tabelle

Ziehe die Linie zwischen Zeichnung und Tabelle, um einem von beiden mehr Platz zu geben. Mit der
Tastatur fokussierst du die Linie und drückst ↑ oder ↓. Dimo merkt sich die Höhe.

## Wenn eine Zeichnung sich nicht öffnen lässt

Dimo zeigt über der Zeichnung eine Meldung, zum Beispiel dass die Datei nicht geöffnet werden
konnte. Als Zeichnung lassen sich in dieser Version nur PDF-Dateien verwenden. Sagt die Meldung,
dass die PDF-Komponente fehlt, ist die Installation unvollständig; installiere Dimo erneut.
