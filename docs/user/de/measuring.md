# Ergebnisse eingeben

In der Ansicht **Messen** (⌘3) gibst du ein, was du an den Teilen gemessen hast. Die Zeilen sind
die zu prüfenden Merkmale, jede Spalte ist ein Teil (Muster). Dimo vergleicht jeden Wert mit den
Grenzmaßen und markiert ihn als i.O. oder n.i.O.

## Muster

Ein neues Projekt hat keine Muster. Gib unter **Anzahl** ein, wie viele Teile du misst, und wähle
**Muster hinzufügen**. Die Seriennummern zählen von der höchsten reinen Zahl weiter (`1`, `2`,
`3`); als Prüfer steht dein Benutzername aus den Einstellungen.

Klicke auf den Kopf oder eine Zelle einer Musterspalte, um sie zum aktiven Muster zu machen. Die
Seitenleiste rechts zeigt Seriennummer, Datum, Prüfer, Prüfmittel und Bemerkung; dort änderst du
sie. Eine Seriennummer darf nicht leer sein und muss im Projekt eindeutig sein. **Nach links** und
**Nach rechts** ändern die Reihenfolge der Spalten. **Muster löschen** löscht die Spalte; hat das
Muster Ergebnisse, fragt Dimo vorher, weil sie mit gelöscht werden. Jede Änderung machst du mit
⌘Z rückgängig.

## Die Tabelle

Aufgelistet sind nur Merkmale mit eingeschaltetem **Prüfen**, in der Reihenfolge der Nummern.
Hilfsmaße, theoretisch genaue Maße und abgelehnte Merkmale fehlen. Die linken Spalten bleiben
stehen, wenn du seitlich scrollst: Nummer, Anforderung, Nennmaß, unteres und oberes Grenzmaß,
Einheit, Ergebnisart und eine Übersicht der Zeile. Grenzmaße aus einer Entwurfstabelle tragen die
Entwurfsmarke.

| Spalte | Bedeutung |
|---|---|
| Ergebnisart | **Automatisch** (Messwert, wenn das Merkmal ein Grenzmaß hat, sonst i.O. / n.i.O.), **Messwert** oder **i.O. / n.i.O.** |
| Übersicht | Wie viele Ergebnisse der Zeile i.O., n.i.O., nicht bewertet sind oder fehlen |
| Musterkopf | Seriennummer des Teils und dieselben Zahlen für dieses Teil |

Die Zeile über der Tabelle zeigt das Gesamtergebnis aller Teile und warum Dimo die letzte Eingabe
nicht angenommen hat.

Ein Dreieck neben der Nummer bedeutet: Grenzmaße oder Ergebnisart dieses Merkmals wurden nach dem
letzten freigegebenen Bericht geändert. Die Ergebnisse bleiben; ihre Bewertung folgt den neuen
Grenzmaßen.

## Ergebnisse tippen

Die Tabelle ist dafür gemacht, ein Teil von oben nach unten ohne Maus zu messen.

1. Klicke auf die erste Zelle des Teils oder geh mit den Pfeiltasten dorthin.
2. Tippe den Wert. Mit dem ersten Zeichen beginnt die Eingabe; Punkt und Komma gehen beide
   (`12.05`, `12,05`), und die getippten Stellen bleiben erhalten (`12.10` bleibt `12.10`).
3. Drücke **Enter**. Dimo speichert den Wert und geht zum nächsten Merkmal.

**Shift+Enter** geht stattdessen nach oben, **Tab** und **Shift+Tab** bestätigen und gehen zum
nächsten oder vorherigen Teil; nach dem letzten Teil führt Tab aus der Tabelle zur Seitenleiste. Die Pfeiltasten bewegen, ohne etwas zu ändern, **F2** bearbeitet
den Wert in der Zelle, **Entf** löscht ihn und **Escape** bricht die Eingabe ab. Nimmt Dimo einen
Wert nicht an (zum Beispiel Buchstaben in einer Messwertzelle), öffnet sich die Zelle wieder und
die Zeile über der Tabelle sagt, warum.

Bei einem Merkmal mit der Ergebnisart **i.O. / n.i.O.** drückst du **O** oder **+** für i.O. und
**N** oder **X** für n.i.O.; Dimo speichert es und geht nach unten. **Leertaste** wechselt
zwischen i.O. und n.i.O. `i.O.` kannst du auch als Wort tippen. Für die Wörter `n.i.O.`, `ok`
oder `not ok` drückst du zuerst **F2**, weil **N** und **O** allein das Ergebnis schon setzen.

Werte gibst du in der Einheit des Merkmals ein, die in der Spalte **Einheit** steht.

## Einen Bereich einfügen

Kopiere einen Bereich von Werten aus einer Tabellenkalkulation oder einem Messprogramm, klicke auf
die Zelle, in der der Bereich beginnt, und drücke ⌘V. Die Zeilen des Bereichs gehen die Merkmale
hinunter, die Spalten über die Teile. Der ganze Bereich ist ein Schritt: ⌘Z entfernt ihn wieder. Eine leere Zelle im Bereich löscht das
Ergebnis in dieser Zelle.
Passt ein Wert nicht, wird nichts eingefügt. Reicht der Bereich über das letzte Merkmal oder das
letzte Teil hinaus, sagt Dimo das und fügt nichts ein; füge zuerst Muster hinzu.

## Markierungen

Jede Bewertung hat ihre eigene Form, so lässt sich die Tabelle auch ohne Farben und auf einem
Schwarzweißausdruck lesen.

| Zeichen | Bewertung | Bedeutung |
|---|---|---|
| ✓ | i.O. | Innerhalb der Grenzmaße (ein Wert gleich einem Grenzmaß ist i.O.), oder i.O. eingegeben |
| ✗ mit fettem Rand | n.i.O. | Außerhalb eines Grenzmaßes, oder n.i.O. eingegeben |
| ○ | Nicht bewertet | Ein Messwert für ein Merkmal ohne Grenzmaße |
| leere Zelle | Fehlt | Noch kein Ergebnis |

Ein Teil ist i.O., wenn jedes Merkmal i.O. ist, n.i.O., wenn eines n.i.O. ist, und sonst offen.

## Ergebnisart

Dimo wählt die Ergebnisart selbst (**Automatisch**): einen Messwert für Merkmale mit Grenzmaßen,
i.O. / n.i.O. für Gewinde, Hinweise und andere Merkmale ohne Grenzmaße. Ändere sie in der Spalte
**Ergebnisart** der Tabelle oder in der Detailansicht der Zeichnungsansicht. Das erste Ergebnis
legt die Art fest, damit spätere Änderungen der Grenzmaße ein Messwertmerkmal nicht zu einem
i.O.-/n.i.O.-Merkmal machen. Solange ein Merkmal Ergebnisse der einen Art hat, ändert Dimo es
nicht auf die andere Art; lösche zuerst die Ergebnisse.
