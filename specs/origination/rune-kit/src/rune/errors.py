class RuneError(Exception):
    def __init__(self, message: str, path: str | None = None):
        super().__init__(message)
        self.path = path

class TypeError(RuneError):
    pass

class StratificationError(RuneError):
    pass

class ConflictError(RuneError):
    pass
