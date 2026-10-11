port module Hello exposing (main)
import Platform
import Platform.Cmd as Cmd
import Platform.Sub as Sub
import Round
port output : String -> Cmd msg
main =
    Platform.worker
        { init = \_ -> ( (), output (Round.round 9 1.2) )
        , update = \_ model -> ( model, Cmd.none )
        , subscriptions = \_ -> Sub.none
        }
