# Ergebnisse eingeben

Im Reiter **Messen** (⌘3) gibst du ein, was du an den Teilen gemessen hast. Die Zeilen sind
die zu prüfenden Merkmale, jede Spalte ist ein Teil. Dimo vergleicht jeden Wert mit den
Grenzmaßen und markiert ihn als i.O. oder n.i.O. Teile, Ergebnisse und Bewertungen werden im
Projekt gespeichert und stehen im [Prüfbericht](exports.md).

## Teile

Ein Teil ist ein Stück, das du misst. Ein neues Projekt hat keine Teile. Gib unter **Anzahl** ein,
wie viele Teile du misst (1 bis 100 auf einmal), und wähle **Teile hinzufügen** im Reiter
Messen. Ein Projekt kann
bis zu 500 Teile enthalten. Die Seriennummern zählen von der höchsten reinen Zahl weiter (`1`, `2`,
`3`); das Datum ist leer, als Prüfer steht dein Benutzername aus den **Optionen**.

Klicke auf den Kopf oder eine Zelle einer Teilespalte, um sie zum aktiven Teil zu machen. Die
Eigenschaftenbereich rechts zeigt Seriennummer, Datum, Prüfer, Prüfmittel und Kommentar; dort änderst du
sie. Darunter zählt sie die Ergebnisse dieses Teils: i.O., n.i.O., nicht bewertet und fehlend.
Projekt- und Berichtsdaten findest du unter **Datei > Info**. Eine Seriennummer wird von Leerzeichen am Rand befreit, darf nicht leer sein und muss im
Projekt eindeutig sein (Groß- und Kleinschreibung zählt als Unterschied). Seriennummern können
beliebiger Text sein, zum Beispiel `A-17`. **Nach links** und **Nach rechts** ändern die
Reihenfolge der Spalten, die auch die Reihenfolge in den Berichten ist. **Teil löschen** löscht
die Spalte; hat das Teil Ergebnisse, fragt Dimo vorher, weil sie mit gelöscht werden. Jede
Änderung machst du mit ⌘Z rückgängig. Der Reiter hat auch **Werte einfügen** und **Zelle
leeren** für die aktive Zelle, **Ergebnis dieser Zeile** für die aktive Zeile, und **Nur fehlende
Werte** und **Nur n.i.O.**, um weniger Zeilen zu zeigen; die Zeile, in der du bist, bleibt sichtbar.

## Die Tabelle

Aufgelistet sind nur Merkmale mit eingeschaltetem **Prüfen**, in der Reihenfolge der Nummern.
Hilfsmaße, theoretisch genaue Maße und abgelehnte Merkmale fehlen, in der Tabelle wie in beiden
Berichten. Die linken Spalten bleiben stehen, wenn du seitlich scrollst: Nummer, Anforderung,
Nennmaß, untere und obere Grenze, Einheit, Ergebnisart und eine Übersicht der Zeile. Grenzmaße aus
einer Entwurfstabelle tragen die Entwurfsmarke. Hat ein Merkmal nur eine obere oder nur eine untere
Grenze, bleibt die andere Zelle leer.

| Spalte | Bedeutung |
|---|---|
| Ergebnisart | **Automatisch** (Messwert, wenn das Merkmal ein Grenzmaß hat, sonst i.O. / n.i.O.), **Messwert** oder **i.O. / n.i.O.** |
| Übersicht | Wie viele Ergebnisse der Zeile i.O., n.i.O., nicht bewertet sind oder fehlen |
| Teilekopf | Seriennummer des Teils und dieselben Zahlen für dieses Teil |

Die Zeile über der Tabelle zeigt das Gesamtergebnis aller Teile und warum Dimo die letzte Eingabe
nicht angenommen hat. Die Statusleiste unten zeigt die Summen aller Teile und das Gesamtergebnis.

## Die Messleiste

Die Leiste über der Tabelle funktioniert wie die Bearbeitungszeile einer Tabellenkalkulation. Sie
nennt die aktive Zelle (Merkmalsnummer und Teil), zeigt die Anforderung mit ihren Grenzmaßen und der
Toleranzregel und hat ein Feld **Wert** mit dem Wert der aktiven Zelle. Daneben sagt Dimo das
Ergebnis in Worten, zum Beispiel „i.O., auf der unteren Grenze“ oder „n.i.O., über der oberen
Grenze“.

Du kannst einen Wert in der Zelle oder im Feld **Wert** tippen; beide zeigen beim Tippen denselben
Text. **Enter** im Feld speichert den Wert und geht zum nächsten Merkmal, wie in der Tabelle;
**Umschalt+Enter** geht nach oben und **Escape** bricht ab. Klickst du in das Feld, während du in
einer Zelle tippst, geht dieselbe Eingabe im Feld weiter.

Ein Dreieck neben der Nummer bedeutet: Grenzmaße oder Ergebnisart dieses Merkmals wurden nach dem
letzten ausgegebenen Bericht geändert ([Wenn sich Grenzmaße ändern](#wenn-sich-grenzmaße-ändern)).

Ein Merkmal mit einer Anzahl wie `4X` und einem Ballon hat ein Ergebnis je Teil: gib den
schlechtesten Wert ein. Willst du einen Wert je Element, nummeriere die Elemente einzeln mit
Unternummern ([Nummerierungsstrategien, Zonen und Ansichten](numbering.md)).

## Ergebnisse tippen

Die Tabelle ist dafür gemacht, ein Teil von oben nach unten ohne Maus zu messen.

1. Klicke auf die erste Zelle des Teils oder geh mit den Pfeiltasten dorthin.
2. Tippe den Wert. Mit dem ersten Zeichen beginnt die Eingabe; Punkt und Komma gehen beide
   (`12.05`, `12,05`), und die getippten Stellen bleiben erhalten (`12.10` bleibt `12.10`). Das
   Minuszeichen darf ein Bindestrich oder das Zeichen `−` sein.
3. Drücke **Enter**. Dimo speichert den Wert und geht zum nächsten Merkmal.

**Shift+Enter** geht stattdessen nach oben, **Tab** und **Shift+Tab** bestätigen und gehen zum
nächsten oder vorherigen Teil; nach dem letzten Teil führt Tab aus der Tabelle in den Eigenschaftenbereich.
Die Pfeiltasten bewegen, ohne etwas zu ändern, **F2** bearbeitet den Wert in der Zelle, **Entf**
löscht ihn und **Escape** bricht die Eingabe ab. Nimmt Dimo einen Wert nicht an (zum Beispiel
Buchstaben in einer Messwertzelle), öffnet sich die Zelle wieder und die Zeile über der Tabelle
sagt, warum. Dimo speichert den Wert immer genau so, wie du ihn tippst, und rundet nie.

Werte gibst du in der Einheit des Merkmals ein, die in der Spalte **Einheit** steht. Dimo rechnet
nichts um: tippe Millimeter bei einem Millimetermerkmal, Zoll bei einem Zollmerkmal und Dezimalgrad
bei einem Winkel (Grad und Minuten werden noch nicht angenommen).

### i.O. / n.i.O.

Bei einem Merkmal mit der Ergebnisart **i.O. / n.i.O.** drückst du **O** oder **+** für i.O. und
**N** oder **X** für n.i.O.; Dimo speichert es und geht nach unten. **Leertaste** wechselt
zwischen i.O. und n.i.O.

Diese Wörter gehen auch, in beliebiger Schreibweise: `ok`, `o`, `+`, `io`, `i.O.` für i.O. und
`not ok`, `not_ok`, `nok`, `n`, `x`, `nio`, `n.i.O.` für n.i.O. Das englische `ok` und das deutsche
`i.O.` funktionieren also in beiden Sprachen der App. Weil **O**, **N** und **X** allein das Ergebnis
schon setzen, drückst du zuerst **F2**, um ein längeres Wort wie `n.i.O.` zu tippen. Dieselben
Wörter gehen in einem eingefügten Bereich. Eine Zahl in einer i.O.-/n.i.O.-Zelle oder ein Wort in
einer Messwertzelle wird abgelehnt.

## Einen Bereich einfügen

Kopiere einen Bereich von Werten aus einer Tabellenkalkulation oder einem Messprogramm, klicke auf
die Zelle, in der der Bereich beginnt, und drücke ⌘V. Die Zeilen des Bereichs gehen die Merkmale
hinunter, die Spalten über die Teile. Der ganze Bereich ist ein Schritt: ⌘Z entfernt ihn wieder.
Eine leere Zelle im Bereich löscht das Ergebnis in dieser Zelle. Passt ein Wert nicht, wird nichts
eingefügt. Reicht der Bereich über das letzte Merkmal oder das letzte Teil hinaus, sagt Dimo das
und fügt nichts ein; füge zuerst Teile hinzu.

## Markierungen

Jede Bewertung hat ihre eigene Form, so lässt sich die Tabelle auch ohne Farben und auf einem
Schwarzweißausdruck lesen. Die Berichte verwenden dieselben Zeichen.

| Zeichen | Bewertung | Bedeutung |
|---|---|---|
| ✓ | i.O. | Innerhalb der Grenzmaße (ein Wert gleich einem Grenzmaß ist i.O.), oder i.O. eingegeben |
| ✗ mit fettem Rand | n.i.O. | Außerhalb eines Grenzmaßes, oder n.i.O. eingegeben |
| ○ | Nicht bewertet | Ein Messwert für ein Merkmal ohne Grenzmaße |
| leere Zelle | Fehlt | Noch kein Ergebnis |

Für ein ganzes Teil, eine Zeile oder den Bericht gibt es drei Bewertungen: **n.i.O.**, wenn ein
Ergebnis n.i.O. ist, **i.O.**, wenn jedes geprüfte Merkmal i.O. ist, und sonst **offen** (etwas
fehlt oder ist nicht bewertet, oder es gibt gar keine Ergebnisse).

## So entscheidet Dimo

Dimo vergleicht den eingegebenen Wert mit den Grenzmaßen, wie sie jetzt sind. Von der Bewertung
wird nichts gespeichert, sie folgt also immer den aktuellen Grenzmaßen.

- Die Grenzmaße gehören dazu: Ein Wert gleich einem Grenzmaß ist i.O. Der Vergleich ist exakt,
  `12` und `12.000` sind derselbe Wert. Der Wert wird nie gerundet, und eine Messunsicherheit wird
  nicht berücksichtigt.
- Mit nur einem oberen Grenzmaß (zum Beispiel `MAX`) oder nur einem unteren (`MIN`) wird nur dieses
  geprüft.
- Ein Wert für ein Merkmal ganz ohne Grenzmaße ist **nicht bewertet** (○). Er macht das Teil nicht
  n.i.O., aber solange so ein Ergebnis existiert, können Teil und Bericht nicht **i.O.** sein.
- Bei i.O. / n.i.O. ist i.O. bestanden und n.i.O. nicht bestanden. Grenzmaße zählen nicht.
- Kein Ergebnis heißt **fehlt**.
- Grenzmaße aus einer Entwurfstabelle werden wie alle anderen verwendet. Die Tabelle und die
  Berichte kennzeichnen sie als Entwurf ([Toleranzen](tolerances.md)).

## Ergebnisart

Dimo wählt die Ergebnisart selbst (**Automatisch**): einen Messwert für Merkmale mit Grenzmaßen,
i.O. / n.i.O. für Gewinde, Hinweise und andere Merkmale ohne Grenzmaße. Ändere sie in der Spalte
**Ergebnisart** der Tabelle oder in der Detailansicht der Zeichnungsansicht. Das erste Ergebnis
legt die Art fest: Mit dem ersten Ergebnis eines Merkmals mit **Automatisch** setzt Dimo die Art
auf Messwert oder i.O. / n.i.O., damit spätere Änderungen der Grenzmaße ein Messwertmerkmal nicht
zu einem i.O.-/n.i.O.-Merkmal machen. Löschen der Ergebnisse macht das nicht rückgängig.

Dimo wandelt Ergebnisse nie um. Solange ein Merkmal Ergebnisse der einen Art hat, lehnt Dimo die
Änderung auf die andere Art ab, und es lehnt auch ab, wieder **Automatisch** zu setzen. Die Zeile
über der Tabelle sagt, dass du zuerst die Ergebnisse löschen sollst. Lösche sie mit **Entf** in
jeder Zelle und ändere dann die Art.

## Wenn sich Grenzmaße ändern

Grenzmaße ändern sich, wenn du Nennmaß, ein Abmaß oder ein Grenzmaß bearbeitest, eine Anforderung
neu einliest, die allgemeine Toleranz oder die Regeln des Projekts änderst oder ein Merkmal in der
Tabelle bearbeitest.

- Die Ergebnisse bleiben. Dimo löscht oder wandelt sie nicht um.
- Die Bewertungen folgen sofort den neuen Grenzmaßen, in der Tabelle und in jedem Bericht, den du
  danach exportierst. Ein Wert, der i.O. war, kann jetzt n.i.O. sein, und umgekehrt.
- Ein Merkmal, das schon Ergebnisse hat, behält seine Ergebnisart (siehe oben).
- Ein Bericht, der früher exportiert wurde, ist eine Datei und ändert sich nicht. Sind die neuen
  Grenzmaße wichtig, exportiere den Bericht noch einmal, als ausgegeben, wenn er an einen Kunden
  geht.
- Ein Dreieck neben der Nummer in der Tabelle zeigt jedes Merkmal, dessen Grenzmaße oder
  Ergebnisart sich vom Stand beim letzten ausgegebenen Bericht unterscheiden. Grenzmaße gelten als
  verschieden, wenn sich der Wert oder die Schreibweise unterscheidet (`12` gegen `12.0`). Die
  Marke verschwindet, wenn die Änderung rückgängig gemacht oder die alten Grenzmaße wieder
  eingegeben werden. Ein Merkmal, das du nach dem letzten ausgegebenen Bericht hinzugefügt hast,
  ist nicht markiert. Vor dem ersten ausgegebenen Bericht ist nichts markiert. Vor dem nächsten
  Export als ausgegeben warnt Dimo ebenfalls vor solchen Änderungen ([Exporte](exports.md)).
- Löschst du ein Merkmal, werden seine Ergebnisse im selben Schritt gelöscht, ohne Rückfrage, und
  ⌘Z bringt beides zurück. Beim Löschen eines Teils fragt Dimo vorher, wenn es Ergebnisse hat. Dasselbe gilt für Merkmale mit Ergebnissen: Vor dem Löschen sagt Dimo, wie viele Ergebnisse mit verschwinden; ⌘Z holt sie zurück.
