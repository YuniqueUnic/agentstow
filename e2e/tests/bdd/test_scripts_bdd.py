from __future__ import annotations

from pytest_bdd import scenarios

from .steps.test_common_steps import *  # noqa: F401,F403
from .steps.test_scripts_steps import *  # noqa: F401,F403


scenarios("features/scripts_timeout.feature")
