# Toleranzen

Dimo bestimmt für jedes gelesene Maß das obere und das untere Grenzmaß, speichert, welche Regel
sie ergeben hat, und erklärt diese Regel in Worten. Diese Seite beschreibt die Regeln, die
Toleranzeinstellungen eines Projekts, die Markierungen in der Tabelle und den Änderungsverlauf
eines Merkmals.

## Woher die Grenzmaße kommen

Dimo prüft die Regeln in dieser Reihenfolge und nimmt die erste, die Grenzmaße ergibt:

1. **Explizit**: eine Toleranz in der Zeichnung, etwa `±0.1`, `+0.2 -0.1`, zwei Grenzmaße, `MIN`
   oder `MAX`. Eine eingetragene Toleranz gilt immer.
2. **Passungstabelle**: eine Passung wie `H7` ohne eingetragene Abmaße, aufgelöst mit der Tabelle
   nach ISO 286.
3. **Zeichnungsregel**: eine eigene Tabelle des Projekts, die als Zeichnungsregel gewählt ist.
4. **Allgemeintoleranz**: Norm und Toleranzklasse des Projekts, nach Nennmaßbereich.
5. **Dezimalstellenregel**: eine Toleranz je Anzahl der Dezimalstellen, mit denen das Maß
   geschrieben ist.

Greift keine Regel, erhält das Merkmal die Regel **Keine Toleranz festgelegt** und keine
Grenzmaße; prüfe es und trage die Grenzmaße in der Tabelle ein. Grenzmaße, die du selbst
eingibst oder änderst, erhalten die Regel **Von Hand eingegeben**. Dimo ändert sie nie wieder von
sich aus.

Steht bei einer Passung ein Abmaß, das von der Passungstabelle abweicht, gilt das eingetragene
Abmaß, und das Merkmal erhält einen Hinweis.

## Toleranzeinstellungen des Projekts

Öffne **Einstellungen** (⌘,), während ein Projekt offen ist. Der Abschnitt **Toleranzen des
Projekts** enthält die Regeln dieses Projekts. Sie werden in der Projektdatei gespeichert, und
jede Änderung ist ein Rückgängig-Schritt.

- **Allgemeintoleranz**: Tabelle und Klasse für Maße ohne Toleranz, zum Beispiel ISO 2768-1
  Klasse m. Passungstabellen stehen hier nicht zur Wahl.
- **Zeichnungsregel (eigene Tabelle)**: eine eigene Tabelle des Projekts, die vor der
  Allgemeintoleranz gilt.
- **Regeln nach Dezimalstellen**: eine Toleranz je Anzahl Dezimalstellen, zum Beispiel 2 Stellen
  ±0,05. Gib Stellen und Toleranz ein und wähle **Regel hinzufügen**. Jede Anzahl Stellen hat eine
  Regel.
- **Rundung nach Einheitenumrechnung**: Dezimalstellen für Werte, die in mm oder Zoll umgerechnet
  werden.
- **Eigene Tabellen**: **Tabelle importieren…** liest eine eigene Tabellendatei (TOML, im Format
  der mitgelieferten Tabellen). Dimo prüft die ganze Datei, bevor sie verwendet wird. Wird sie
  abgelehnt, steht der Grund da, mit der Zeile der Datei, wo Dimo sie bestimmen kann. Eine
  importierte Tabelle wird in das Projekt kopiert, das Projekt braucht die Datei danach nicht
  mehr, und sie bleibt nach einem Absturz erhalten, auch vor dem Speichern. Wird eine Tabelle mit
  derselben Kennung erneut importiert, ersetzt sie die alte, und die Einstellungen, die sie
  verwenden, wechseln auf die neue Version.

Eine Änderung der Einstellungen ändert die Grenzmaße der Merkmale nicht, die schon im Projekt
sind. Neue Merkmale, die Rahmenauswahl und eingetippte Werte verwenden die neuen Einstellungen.

## Nach einer Änderung neu auswerten

Um neue Einstellungen auf bestehende Merkmale anzuwenden, wähle sie aus (in der Tabelle oder in
der Zeichnung) und wähle **Ausgewählte neu auswerten** in der Seitenleiste. Dimo liest die
Anforderung jedes Merkmals mit den aktuellen Einstellungen neu und setzt Nennmaß, Einheit,
Abmaße, Grenzmaße, Passung und Regel. Das Ganze ist ein Rückgängig-Schritt.

- Merkmale mit von Hand eingegebenen Grenzmaßen bleiben, wie sie sind.
- Merkmale, deren Text leer oder kein Maß ist, bleiben unverändert.
- Art, Anzahl und das Feld **Prüfen** bleiben so, wie du sie gesetzt hast. Ein Merkmal, dessen
  Text noch nie gelesen wurde, erhält sie aus dem Text.

Die Zeile unter der Schaltfläche sagt, wie viele neu gelesen, behalten oder nicht lesbar waren.

## Regel, Erklärung und Markierungen

Die Spalte **Regel** der Merkmalstabelle zeigt die Regel jedes Merkmals. Wähle ein Merkmal aus,
um Regel, Grenzmaße und Erklärung in der Seitenleiste zu sehen, Reiter **Toleranz**. Die
Erklärung nennt Tabelle, Klasse und Nennmaßbereich, aus denen der Wert stammt, in der Sprache der
App. Die Regel wird auch in die CSV- und Excel-Liste exportiert ([Exporte](exports.md)).

Markierungen neben der Regel. Jede hat ihre eigene Form, sie sind also auch ohne Farbe zu
unterscheiden:

| Markierung | Bedeutung |
|---|---|
| **Entwurf** in gestricheltem Rahmen | Die Grenzmaße stammen aus einer Tabelle, deren Werte noch nicht geprüft sind |
| Kreis mit Balken | Keine Toleranz festgelegt: keine Grenzmaße. Dieses Merkmal prüfen |
| Dreieck mit ! | Ein Hinweis, zum Beispiel eingetragene Abmaße, die von der Passungstabelle abweichen. Zeige darauf, um den Hinweis zu lesen |
| **(Hilfsmaß)** mit runden Enden | Hilfsmaß: keine Grenzmaße |
| **Theor. genau** in eckigem Rahmen | Theoretisch genaues Maß: keine Grenzmaße |

## Entwurfstabellen

Die mitgelieferten Tabellen sind Entwürfe, bis ihre Werte mit der gedruckten Norm verglichen
wurden. Grenzmaße aus einer Entwurfstabelle funktionieren wie alle anderen, tragen aber die
Markierung **Entwurf** in der Tabelle, in der Seitenleiste und auf der Karte der Rahmenauswahl.
Auch eine importierte eigene Tabelle kann ein Entwurf sein.

## Hilfsmaße und theoretisch genaue Maße

Hilfsmaße (in Klammern oder mit `REF`) und theoretisch genaue Maße (im Rahmen) erhalten keine
Grenzmaße und werden nicht geprüft: Ihr Feld **Prüfen** ist zu Beginn leer. Du kannst es in der
Tabelle oder in der Seitenleiste ankreuzen, wenn du sie trotzdem messen willst; das Neuauswerten
behält deine Wahl.

## Verlauf eines Merkmals

Der Reiter **Verlauf** in der Seitenleiste listet jede Änderung des ausgewählten Merkmals, die
neueste zuerst, aus dem Änderungsprotokoll des Projekts. Jeder Eintrag zeigt wann, wer und die
Quelle der Änderung:

- **Von Hand**: du hast dieses Merkmal geändert.
- **Durch Regel**: eine Regel hat es geändert, zum Beispiel das Neunummerieren nach einem Löschen
  oder ein Neuauswerten.
- **Erkennung**: es stammt aus einer übernommenen Karte der Rahmenauswahl.

Der Eintrag nennt die geänderten Werte, oder dass das Merkmal angelegt oder entfernt wurde.
Rückgängig und Wiederherstellen erscheinen als eigene Einträge.
