"""Bind the bounded local service under the caller's actual mutex/pin review."""
from pathlib import Path
import sys
from types import SimpleNamespace


def bind(original, guard, prerequisite_pins, packet_pins):
    reviewed_v7 = original.HERE / 'revisions/private-v7/owned_service_v7.py'
    local = Path(__file__).resolve().parent / 'owned_service_work_v2.py'
    assert str(reviewed_v7) in prerequisite_pins
    assert str(local) in packet_pins
    guard.verify({str(reviewed_v7): prerequisite_pins[str(reviewed_v7)],
                  str(local): packet_pins[str(local)]})
    assert 'owned_service_work_v2' not in sys.modules, 'Unreviewed cached mutable work service'
    import owned_service_work_v2 as service
    assert Path(service.__file__).resolve() == local
    guard.verify({str(local): packet_pins[str(local)]})
    facade = SimpleNamespace(**vars(original))
    facade.owned_command = lambda g, h, cmd, env, out, fd, timeout, name, **kwargs: service.owned_command(
        original, g, h, cmd, env, out, fd, timeout, name, **kwargs)
    facade.native_images = lambda g, h, binary, env, out, fd, roots: service.native_images(
        original, g, h, binary, env, out, fd, roots)
    facade.reviewed_bounded_service_origin = str(reviewed_v7)
    return facade
