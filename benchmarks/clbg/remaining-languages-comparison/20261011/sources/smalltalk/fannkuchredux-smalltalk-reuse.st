"Computer Language Benchmarks Game. Local native Pharo adaptations, 2026."
Object subclass: #BenchmarksGame
    instanceVariableNames: ''
    classVariableNames: ''
    poolDictionaries: ''
    category: 'Benchmarks-Game'!
!BenchmarksGame class methodsFor: 'application'!

do: n
    | perm count scratch maximum checksum index flips first left right temp r rotated more |
    perm := (1 to: n) asArray.
    count := Array new: n withAll: 0.
    scratch := Array new: n.
    maximum := 0.
    checksum := 0.
    index := 0.
    more := true.
    [more] whileTrue: [
        flips := 0.
        (perm at: 1) = 1 ifFalse: [
            scratch replaceFrom: 1 to: n with: perm startingAt: 1.
            [(first := scratch at: 1) = 1] whileFalse: [
                left := 1.
                right := first.
                [left < right] whileTrue: [
                    temp := scratch at: left.
                    scratch at: left put: (scratch at: right).
                    scratch at: right put: temp.
                    left := left + 1.
                    right := right - 1].
                flips := flips + 1]].
        checksum := index even ifTrue: [checksum + flips] ifFalse: [checksum - flips].
        maximum := maximum max: flips.
        index := index + 1.
        r := 2.
        rotated := false.
        [r <= n and: [rotated not]] whileTrue: [
            temp := perm at: 1.
            1 to: r - 1 do: [:i | perm at: i put: (perm at: i + 1)].
            perm at: r put: temp.
            count at: r put: ((count at: r) + 1) \\ r.
            (count at: r) = 0 ifTrue: [r := r + 1] ifFalse: [rotated := true]].
        more := rotated].
    Stdio stdout print: checksum;
        lf;
        nextPutAll: 'Pfannkuchen(';
        print: n;
        nextPutAll: ') = ';
        print: maximum;
        lf
! !
