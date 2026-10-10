# Nummerierungsstrategien, Zonen und Ansichten

Dimo nummeriert die Ballons in der Reihenfolge der Merkmalstabelle. Der Bereich **Nummerierung**
rechts bringt diese Reihenfolge in ein Muster: Blatt für Blatt und Zone für Zone, Ansicht für
Ansicht, im Uhrzeigersinn um jede Ansicht oder nach Art. Du siehst die neuen Nummern, bevor sie
sich ändern, und das Anwenden ist ein Schritt, den ⌘Z zurücknimmt.

## Zonenraster

Die meisten Zeichnungsrahmen sind in Zonen eingeteilt, mit Buchstaben an einem Rand und Zahlen am
anderen, zum Beispiel `B3`. Dimo liest den Rahmen noch nicht selbst, darum legst du das Raster
einmal je Blatt in den **Blatteigenschaften** unter **Zonenraster** fest:

1. Wähle **Rahmen zeichnen** und ziehe ein Rechteck entlang der Innenkante des Zeichnungsrahmens
   auf. Esc bricht ab. (**Zonenraster hinzufügen** setzt stattdessen einen Rahmen 10 mm innerhalb
   des Blattrands, den du später neu zeichnen kannst.)
2. Stelle die Zahl der **Spalten** und **Zeilen** ein.
3. Wähle, wie die Bezeichnungen geschrieben sind: für Spalten 1, 2, 3 oder A, B, C von links oder
   von rechts, für Zeilen A, B, C oder 1, 2, 3 von oben oder von unten.
4. Verwendet der Rahmen andere Bezeichnungen, gib sie in den Bezeichnungsfeldern ein, mit Kommas
   getrennt, von links nach rechts und von oben nach unten wie gedruckt. Die Anzahl der
   Bezeichnungen legt die Zahl der Spalten und Zeilen fest.

Das Raster wird mit gestrichelten Linien und seinen Bezeichnungen auf dem Blatt gezeigt.
**Zonenraster entfernen** löscht es.

## Ansichten

Die Strategien je Ansicht brauchen die Ansichten der Zeichnung. Wähle unter **Ansichten**
**Ansicht zeichnen** und ziehe ein Rechteck um eine Ansicht auf, zum Beispiel die Vorderansicht
oder einen Schnitt. Zeichne die Ansichten in der Reihenfolge, in der sie nummeriert werden
sollen. Jede Ansicht kann einen Namen wie `A-A` oder `Einzelheit B` erhalten; die Schaltfläche ×
entfernt sie. Ansichten werden als gepunktete Rechtecke auf dem Blatt gezeigt.

## Strategien

Ein Ballon gehört zum Ort seines Hinweislinienendes, dem Punkt am Element. Gelesen wird so, wie
das Blatt auf dem Bildschirm gezeigt wird; ein gedrehtes Blatt wird so gelesen, wie du es siehst.

| Strategie | Reihenfolge |
|---|---|
| Blatt, Zone, Lesereihenfolge | Blatt für Blatt, Zone für Zone (Zeilen von Zonen von oben nach unten, von links nach rechts), in einer Zone in Lesereihenfolge. Ohne Zonenraster wird das ganze Blatt in Lesereihenfolge gelesen. Das ist die Vorgabe für neue Projekte. |
| Je Ansicht, Lesereihenfolge | Ansicht für Ansicht in der Reihenfolge, in der du sie gezeichnet hast, in jeder Ansicht in Lesereihenfolge. Ballons außerhalb aller Ansichten kommen zuletzt auf ihrem Blatt, in Lesereihenfolge. |
| Je Ansicht, im Uhrzeigersinn | Ansicht für Ansicht, im Uhrzeigersinn um die Mitte der Ansicht, beginnend bei 12 Uhr. Ballons außerhalb aller Ansichten kommen zuletzt. |
| Nach Art | Nach Art gruppiert (Längenmaß, Durchmesser, Radius, Winkel und so weiter), jede Art in Blatt- und Zonenreihenfolge. |
| Manuell (aktuelle Reihenfolge) | Behält die Reihenfolge der Tabelle. Du änderst sie durch Ziehen von Zeilen. |

**Lesereihenfolge** heißt von oben nach unten, dann von links nach rechts. Ballons, deren
Hinweislinienenden in der Höhe weniger als einen Ballondurchmesser auseinander liegen, gelten als
eine Zeile und werden von links nach rechts gelesen. Ein Punkt in zwei Ansichten, zum Beispiel
eine Einzelheit innerhalb einer größeren Ansicht, gehört zur kleineren. Ein Punkt außerhalb des
Zonenrahmens gehört zur nächsten Zone.

## Vorschau und Anwenden

1. Wähle die Strategie im Bereich **Nummerierung**. Ein kurzer Text erklärt sie.
2. Setze den Haken bei **Neue Nummern als Vorschau zeigen**. Jeder Ballon erhält seine neue
   Nummer in einem gestrichelten Kasten daneben, und die Tabelle zeigt sie neben der aktuellen
   Nummer. Nummern, die sich ändern, sind fett und farbig, Nummern, die bleiben, dünn und grau. Der
   Bereich zählt, wie viele Nummern sich ändern.
3. Wähle **Nummerierung anwenden**. Die Tabelle wird in die neue Reihenfolge gebracht, alle
   Ballons erhalten ihre neuen Nummern, und die Strategie wird mit dem Projekt gespeichert. ⌘Z
   macht den ganzen Schritt rückgängig.

Die Vorschau folgt jeder Änderung, solange sie eingeschaltet ist, und zeigt so immer, was das
Anwenden bewirken würde. Neue Ballons werden weiter am Ende mit der nächsten Nummer angefügt;
wende die Strategie erneut an, wenn du mit dem Setzen fertig bist.

## Wiederholte Elemente

Eine Angabe wie `4X Ø5` steht für mehrere Elemente. **Wiederholte Elemente (4X)** legt fest, wie
sie nummeriert werden:

| Einstellung | Ergebnis |
|---|---|
| Ein Ballon mit Anzahl | Ein Merkmal mit Anzahl 4 und einer Nummer. Das ist die Vorgabe. |
| Unternummern (5.1, 5.2) | Ein Merkmal je Element, nummeriert 5.1, 5.2, 5.3, 5.4, mit gestapelten Ballons. |

Mit Unternummern wird eine aus der Zeichnung gelesene Angabe mit Anzahl beim Übernehmen zu einem
Merkmal je Element. Jede Strategie hält diese Merkmale zusammen. Schaltest du zurück auf **Ein
Ballon mit Anzahl**, erhalten sie wieder einfache Nummern; sie bleiben getrennte Merkmale.
Änderst du die Art oder die Anforderung eines davon, erhält es eine eigene einfache Nummer.
Bei gesperrter Nummerierung und bei mehr als 100 Elementen bleibt eine Angabe ein Merkmal mit
ihrer Anzahl.

## Gesperrte Nummerierung

Nach einem Export als freigegeben ist die Nummerierung gesperrt ([Exporte](exports.md)). Die
Nummern ändern sich dann nie, und **Nummerierung anwenden** wird nicht angeboten. **Hinzugefügt
bei Sperre** legt die Nummer eines danach hinzugefügten Merkmals fest:

| Einstellung | Beispiel |
|---|---|
| Nächste freie Nummer | Nach 12 kommt 13 (Vorgabe). |
| Unternummer (12.1) | Ist 12 die höchste Nummer, erhält das neue 12.1, dann 12.2. |
| Buchstabe angehängt (12A) | Ist 12 die höchste Nummer, erhält das neue 12A, dann 12B. |

Eine bei Sperre vergebene Nummer wird nie wieder vergeben, auch wenn du ihr Merkmal löschst.
