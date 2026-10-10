# Eine Zeichnung ballonieren

Ein Ballon ist die nummerierte Markierung auf der Zeichnung, die zu einem Merkmal gehört. Ballons
setzt du in der Zeichnungsansicht mit dem Werkzeug **Ballon setzen**. Alles unten lässt sich mit
⌘Z (Ctrl+Z unter Windows und Linux) rückgängig machen und mit ⇧⌘Z (Ctrl+Shift+Z oder Ctrl+Y)
wiederholen.

## Werkzeuge

| Werkzeug | Taste | Was die linke Maustaste auf der Zeichnung tut |
|---|---|---|
| Auswählen | V | Klick wählt einen Ballon, Ziehen verschiebt Ballons, Ziehen auf leerer Zeichnung verschiebt die Ansicht |
| Ballon setzen | B | Klick setzt einen neuen Ballon, ein gezogener Rahmen liest seinen Text (Rahmenauswahl), Ziehen auf einem Ballon verschiebt ihn |

In beiden Werkzeugen verschiebst du die Ansicht mit der mittleren Maustaste oder mit gedrückter
Leertaste beim Ziehen. Zoomen und Blätter stehen unter [Eine Zeichnung ansehen](drawing-view.md).

## Ballons setzen

1. Drücke **B** oder wähle **Ballon setzen** in der Werkzeugleiste.
2. Klicke auf das Merkmal, das geprüft werden soll. Der Ballon erscheint rechts oberhalb davon
   mit der nächsten freien Nummer, seine Bezugslinie zeigt auf die Klickstelle. Am Blattrand
   weicht er auf die andere Seite aus.
3. Neben dem Ballon öffnet sich ein kleines Feld. Tippe den Wert so, wie er auf der Zeichnung
   steht, zum Beispiel `Ø8 f7` oder `100 ±0.2`.
4. Drücke **Enter**. Der Wert ist gespeichert und das Werkzeug bleibt aktiv, du klickst also
   gleich das nächste Merkmal an.

Von Hand geht es am schnellsten so: klicken, tippen, Enter, klicken, tippen, Enter.

- **Escape** im Feld schließt es, ohne den Text zu speichern. Der Ballon bleibt; ⌘Z entfernt ihn.
- Ein Klick an eine andere Stelle speichert das Getippte ebenfalls.
- **Enter** auf einem ausgewählten Ballon oder ein Doppelklick darauf öffnet das Feld wieder.

Dimo liest den getippten Text wie ein Maß auf der Zeichnung: `Ø30 H7 +0.0203 -0` ergibt die Art
**Durchmesser**, das Nennmaß `30`, die Passung `H7`, die Abmaße und die Grenzwerte `30.0203` und
`30` mit der Regel **Explizit**. Text, der kein Maß ist, etwa ein Hinweis, wird nur als
Anforderung gespeichert. `Ø8 f7` allein erhält seine Grenzwerte aus der Passungstabelle nach
ISO 286 mit der Regel **Passungstabelle**. Werte aus einer Tabelle, die noch ein Entwurf ist,
werden so markiert. Ein Maß ohne Toleranz erhält **Keine Toleranz festgelegt**, weil das Projekt
noch keine Allgemeintoleranz hat; seine Grenzwerte trägst du in der Tabelle ein
([Merkmalstabelle](characteristics.md)).

## Rahmenauswahl

Bei einer Zeichnung mit PDF-Text liest Dimo den Text für dich.

1. Drücke **B** und ziehe einen Rahmen um den Maßtext, gestapelte Toleranzen eingeschlossen.
   Gedrehter Text, etwa entlang einer senkrechten Maßlinie, geht genauso.
2. Neben dem Rahmen öffnet sich eine Karte. Sie zeigt, was Dimo gelesen hat: den
   Anforderungstext, die Art, das Nennmaß, die obere und die untere Grenze, die Regel, aus der
   sie stammen, und eine Erklärung dazu. Aufgedruckte Abmaße gelten immer; weichen sie von der
   Passungstabelle ab, sagt die Erklärung das. Hinweise nennen, was Dimo nicht entscheiden
   konnte, etwa einen Winkel, dessen kürzeren Schenkel es nicht kennt.
3. Prüfe die Werte. Den Anforderungstext (Dimo liest ihn erneut) und die Art kannst du
   korrigieren.
4. Drücke **Enter** oder **Übernehmen**. Der Ballon erscheint rechts oberhalb des Rahmens wie ein
   gesetzter Ballon, mit den Werten der Karte. **Escape** oder **Verwerfen** schließt die Karte,
   ohne etwas zu ändern.

- Vor dem Übernehmen wird nichts hinzugefügt. Übernehmen ist ein Schritt für ⌘Z, auch wenn der
  Rahmen mehrere Maße enthielt.
- Ein Rahmen um mehrere Maße übereinander ergibt für jedes einen Vorschlag.
- Eine Zahl in einem rechteckigen Rahmen wird als theoretisch genaues Maß gelesen, ein Wert in
  Klammern als Hilfsmaß. Beide werden als nicht zu prüfen markiert.
- Kann Dimo den Text nicht als Maß lesen, sagt die Karte das, und Übernehmen speichert nur den
  Text.
- Enthält der Rahmen keinen PDF-Text (gescannte Zeichnung), wird ein Ballon gesetzt und das
  Wertfeld öffnet sich wie bei einem Klick.
- Der Rahmen wird als Quellbereich beim Merkmal gespeichert, so findest du später, woher der
  Wert stammt. Ein einfacher Klick speichert keinen Bereich.

## Ballonnummern

Solange die Nummerierung nicht gesperrt ist, werden die Ballons ohne Lücken mit 1, 2, 3 und so
weiter in der Reihenfolge der Merkmalstabelle nummeriert. Ein neuer Ballon erhält die nächste
Nummer. Löschst du einen Ballon oder verschiebst eine Zeile in der Tabelle, werden alle neu
nummeriert.

Nach einem Export als freigegeben ([Exporte](exports.md)) ist die Nummerierung gesperrt. Die
Nummern ändern sich dann nie: Ein neuer Ballon erhält die höchste je vergebene Nummer plus eins,
eine gelöschte Nummer wird nicht wieder verwendet, und Zeilen lassen sich nicht verschieben.

Wie du nach Zone, Ansicht oder Art nummerierst oder Unternummern für wiederholte Elemente
verwendest, steht unter [Nummerierungsstrategien, Zonen und Ansichten](numbering.md).

## Ballons auswählen

| Aktion | Maus | Taste |
|---|---|---|
| Einen Ballon auswählen | Anklicken | |
| Ballon hinzufügen oder entfernen | Shift+Klick oder ⌘+Klick (Ctrl+Klick) | |
| Mehrere über eine Fläche wählen | Shift+Ziehen auf leerer Zeichnung, Ballons mit dem Mittelpunkt im Rahmen kommen dazu | |
| Alle Ballons des Blatts wählen | | ⌘A (Ctrl+A) |
| Auswahl aufheben | Klick auf leere Zeichnung (Werkzeug Auswählen) | Escape |

Ausgewählte Ballons bekommen einen dicken orangen Ring und einen kleinen quadratischen Griff am
Ende ihrer Bezugslinie.

## Ballons und Bezugslinien verschieben

- Ziehe einen ausgewählten Ballon, um alle ausgewählten Ballons gemeinsam zu verschieben. Ziehst
  du einen nicht ausgewählten Ballon, wird nur dieser gewählt und verschoben. Die Enden der
  Bezugslinien bleiben auf der Zeichnung.
- Ziehe den quadratischen Griff am Ende einer Bezugslinie, um sie auf eine andere Stelle zeigen
  zu lassen.
- Ein Ziehen ist ein Rückgängig-Schritt, egal wie viele Ballons es verschiebt.
- Mit der Tastatur: Shift und eine Pfeiltaste verschieben die ausgewählten Ballons um 1 mm (auf
  dem gedruckten Blatt) in diese Richtung auf dem Bildschirm. Jeder Druck ist ein
  Rückgängig-Schritt. Die Pfeiltasten allein verschieben die Ansicht.

## Stil ändern

Wähle Ballons aus und drücke **S** oder wähle **Stil** in der Werkzeugleiste. Jede Wahl gilt
sofort für alle ausgewählten Ballons:

- **Form**: Kreis, Fahne oder Rechteck.
- **Bezugslinie** ein oder aus.
- **Größe** in Millimetern auf dem gedruckten Blatt. Standard ist 7 mm.
- **Randfarbe**, jede Farbe mit ihrem Namen.
- **Projektstandard verwenden** entfernt die Änderungen, die Ballons folgen wieder dem Projektstil.

Ballons werden in der Größe gezeichnet, die sie auf dem gedruckten Blatt haben. Sie wachsen und
schrumpfen also mit dem Zoom und bleiben immer an ihrer Stelle auf der Zeichnung. Auf einem
gedrehten Blatt bleiben die Nummern aufrecht.

## Ballons löschen

Wähle Ballons aus und drücke **Entfernen** oder **Rücktaste**, oder wähle **Löschen** in der
Werkzeugleiste. Die Merkmale der Ballons werden mit gelöscht. Solange die Nummerierung nicht
gesperrt ist, werden die übrigen Ballons wieder lückenlos ab 1 nummeriert.

## Wie der Status angezeigt wird

Der Status wird nie nur über die Farbe angezeigt:

| Status | Aussehen |
|---|---|
| Angenommen, verifiziert | Durchgehender Rand in der Ballonfarbe |
| Vorgeschlagen | Gestrichelter Rand |
| Abgelehnt | Gestrichelter grauer Rand, Nummer grau, durchgestrichen |

Ballons, die du von Hand setzt, sind angenommen. Vorgeschlagen und abgelehnt kommen mit der
automatischen Erkennung einer späteren Version; die Dateiformate und Exporte können sie schon
verarbeiten (abgelehnte Merkmale erhalten im exportierten PDF keinen Ballon und fehlen in den
Listen).

Alle Tasten stehen unter [Tastenkürzel](shortcuts.md) und in der Übersicht der Tastenkürzel,
drücke **?**.
