import time

if __name__ == "__main__":
    print("Exiting with an error for testing in 1 second")
    time.sleep(1)
    raise Exception("Dummy error")
