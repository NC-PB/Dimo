# Eine Zeichnung ballonieren

Ein Ballon ist die nummerierte Markierung auf der Zeichnung, die zu einem Merkmal gehört. Ballons
setzt du in der Zeichnungsansicht mit dem Werkzeug **Ballon setzen**. Alles unten lässt sich mit
⌘Z (Ctrl+Z unter Windows und Linux) rückgängig machen und mit ⇧⌘Z (Ctrl+Shift+Z oder Ctrl+Y)
wiederholen.

## Werkzeuge

| Werkzeug | Taste | Was die linke Maustaste auf der Zeichnung tut |
|---|---|---|
| Auswählen | V | Klick wählt einen Ballon, Ziehen verschiebt Ballons, Ziehen auf leerer Zeichnung verschiebt die Ansicht |
| Ballon setzen | B | Klick oder Ziehen setzt einen neuen Ballon, Ziehen auf einem Ballon verschiebt ihn |

In beiden Werkzeugen verschiebst du die Ansicht mit der mittleren Maustaste oder mit gedrückter
Leertaste beim Ziehen. Zoomen und Blätter stehen unter [Eine Zeichnung ansehen](drawing-view.md).

## Ballons setzen

1. Drücke **B** oder wähle **Ballon setzen** in der Werkzeugleiste.
2. Klicke auf das Merkmal, das geprüft werden soll, oder ziehe einen Rahmen um seinen Maßtext.
   Der Ballon erscheint rechts oberhalb davon mit der nächsten freien Nummer, seine Bezugslinie
   zeigt auf die Klickstelle (oder auf die Ecke des Rahmens). Am Blattrand weicht er auf die
   andere Seite aus.
3. Neben dem Ballon öffnet sich ein kleines Feld. Tippe den Wert so, wie er auf der Zeichnung
   steht, zum Beispiel `Ø8 f7` oder `100 ±0.2`.
4. Drücke **Enter**. Der Wert ist gespeichert und das Werkzeug bleibt aktiv, du klickst also
   gleich das nächste Merkmal an.

Am schnellsten geht es so: klicken, tippen, Enter, klicken, tippen, Enter.

- **Escape** im Feld schließt es, ohne den Text zu speichern. Der Ballon bleibt; ⌘Z entfernt ihn.
- Ein Klick an eine andere Stelle speichert das Getippte ebenfalls.
- Der gezogene Rahmen wird als Quellbereich beim Merkmal gespeichert, so findest du später, woher
  der Wert stammt. Ein einfacher Klick speichert keinen Bereich.
- **Enter** auf einem ausgewählten Ballon oder ein Doppelklick darauf öffnet das Feld wieder.

Das Feld speichert den Text, wie er geschrieben ist, in der Spalte **Anforderung** der
Merkmalstabelle. Dimo zerlegt ihn in dieser Version nicht in Nennmaß und Toleranzen; Nennmaß,
Abmaße, Art und den Rest trägst du in der Tabelle ein ([Merkmalstabelle](characteristics.md)).

## Ballonnummern

Solange die Nummerierung nicht gesperrt ist, werden die Ballons ohne Lücken mit 1, 2, 3 und so
weiter in der Reihenfolge der Merkmalstabelle nummeriert. Ein neuer Ballon erhält die nächste
Nummer. Löschst du einen Ballon oder verschiebst eine Zeile in der Tabelle, werden alle neu
nummeriert.

Nach einem Export als freigegeben ([Exporte](exports.md)) ist die Nummerierung gesperrt. Die
Nummern ändern sich dann nie: Ein neuer Ballon erhält die höchste je vergebene Nummer plus eins,
eine gelöschte Nummer wird nicht wieder verwendet, und Zeilen lassen sich nicht verschieben.

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
